package main

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"log"
	"math/big"
	"net/http"
	"os"
	"strconv"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const (
	defaultPort        = "5001"
	defaultMaxRoomSize = 4
	pingInterval       = 30 * time.Second
	pongWait           = 60 * time.Second
	writeWait          = 10 * time.Second
)

// ---------------------------------------------------------------------------
// Wire types  (JSON in / out)
// ---------------------------------------------------------------------------

// InMsg is every message sent from a client to the server.
type InMsg struct {
	Type    string          `json:"type"`
	Config  string          `json:"config,omitempty"`  // used by: host, set_config
	Token   string          `json:"token,omitempty"`   // used by: join
	Payload json.RawMessage `json:"payload,omitempty"` // used by: message (any JSON value)
}

// OutMsg is every message sent from the server to a client.
type OutMsg struct {
	Type    string          `json:"type"`
	Token   string          `json:"token,omitempty"`
	ID      string          `json:"id,omitempty"`
	Config  string          `json:"config,omitempty"`
	Members int             `json:"members,omitempty"`
	From    string          `json:"from,omitempty"`
	Payload json.RawMessage `json:"payload,omitempty"`
	Reason  string          `json:"reason,omitempty"`
}

// ---------------------------------------------------------------------------
// safeConn – websocket.Conn with a serialised write path
// ---------------------------------------------------------------------------

type safeConn struct {
	ws  *websocket.Conn
	wmu sync.Mutex
}

func (c *safeConn) write(msg OutMsg) error {
	data, err := json.Marshal(msg)
	if err != nil {
		return err
	}
	c.wmu.Lock()
	defer c.wmu.Unlock()
	_ = c.ws.SetWriteDeadline(time.Now().Add(writeWait))
	return c.ws.WriteMessage(websocket.TextMessage, data)
}

func (c *safeConn) ping() error {
	c.wmu.Lock()
	defer c.wmu.Unlock()
	_ = c.ws.SetWriteDeadline(time.Now().Add(writeWait))
	return c.ws.WriteMessage(websocket.PingMessage, nil)
}

// ---------------------------------------------------------------------------
// Member
// ---------------------------------------------------------------------------

type Member struct {
	id   string
	conn *safeConn
}

func (m *Member) send(msg OutMsg) { _ = m.conn.write(msg) }

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

type Room struct {
	token   string
	host    *Member
	members []*Member
	config  string
	maxSize int
}

func (r *Room) isFull() bool { return len(r.members) >= r.maxSize }

func (r *Room) removeMember(target *Member) {
	for i, m := range r.members {
		if m == target {
			r.members = append(r.members[:i], r.members[i+1:]...)
			return
		}
	}
}

// ---------------------------------------------------------------------------
// Hub  (all mutable state protected by a single mutex)
// ---------------------------------------------------------------------------

type Hub struct {
	mu      sync.Mutex
	rooms   map[string]*Room   // token  → room
	conns   map[*safeConn]*ref // conn   → (room, member)
	maxRoom int
}

type ref struct {
	room   *Room
	member *Member
}

func newHub(maxRoom int) *Hub {
	return &Hub{
		rooms:   make(map[string]*Room),
		conns:   make(map[*safeConn]*ref),
		maxRoom: maxRoom,
	}
}

// generateToken must be called with h.mu held.
func (h *Hub) generateToken() string {
	for {
		n, _ := rand.Int(rand.Reader, big.NewInt(900))
		token := strconv.FormatInt(n.Int64()+100, 10)
		if _, exists := h.rooms[token]; !exists {
			return token
		}
	}
}

func newMemberID() string {
	b := make([]byte, 4)
	_, _ = rand.Read(b)
	return fmt.Sprintf("%x", b)
}

// hostRoom registers conn as the host of a new room.
func (h *Hub) hostRoom(conn *safeConn, config string) (*Room, *Member) {
	h.mu.Lock()
	defer h.mu.Unlock()

	m := &Member{id: newMemberID(), conn: conn}
	room := &Room{
		token:   h.generateToken(),
		host:    m,
		members: []*Member{m},
		config:  config,
		maxSize: h.maxRoom,
	}
	h.rooms[room.token] = room
	h.conns[conn] = &ref{room, m}
	return room, m
}

// joinRoom adds conn to an existing room identified by token.
func (h *Hub) joinRoom(conn *safeConn, token string) (*Room, *Member, error) {
	h.mu.Lock()
	defer h.mu.Unlock()

	room, ok := h.rooms[token]
	if !ok {
		return nil, nil, fmt.Errorf("not_found")
	}
	if room.isFull() {
		return nil, nil, fmt.Errorf("room_full")
	}
	m := &Member{id: newMemberID(), conn: conn}
	room.members = append(room.members, m)
	h.conns[conn] = &ref{room, m}
	return room, m, nil
}

// lookup returns the room and member associated with conn (if any).
func (h *Hub) lookup(conn *safeConn) (*Room, *Member, bool) {
	h.mu.Lock()
	r, ok := h.conns[conn]
	h.mu.Unlock()
	if !ok {
		return nil, nil, false
	}
	return r.room, r.member, true
}

// leaveResult carries everything the caller needs to send notifications
// after the hub mutex has been released.
type leaveResult struct {
	room       *Room
	member     *Member
	roomClosed bool
	remaining  []*Member // snapshot of members still in the room after leaving
}

// leave removes conn from its room (if any) and returns what happened.
// It always removes conn from h.conns. If the host leaves (or the room empties),
// all remaining members are also evicted from h.conns and roomClosed is true.
func (h *Hub) leave(conn *safeConn) *leaveResult {
	h.mu.Lock()
	defer h.mu.Unlock()

	r, ok := h.conns[conn]
	if !ok {
		return nil
	}
	room := r.room
	member := r.member

	delete(h.conns, conn)
	room.removeMember(member)

	hostLeft := member == room.host
	empty := len(room.members) == 0
	roomClosed := hostLeft || empty

	if roomClosed {
		delete(h.rooms, room.token)
		for _, m := range room.members {
			delete(h.conns, m.conn)
		}
	}

	remaining := make([]*Member, len(room.members))
	copy(remaining, room.members)

	// Clear the slice so stale references to this room see no members.
	room.members = nil

	return &leaveResult{
		room:       room,
		member:     member,
		roomClosed: roomClosed,
		remaining:  remaining,
	}
}

// snapshot returns a copy of the room's member list (minus the caller)
// so notifications can be sent after releasing the hub lock.
// Must be called with h.mu held.
func othersSnapshot(room *Room, exclude *Member) []*Member {
	out := make([]*Member, 0, len(room.members))
	for _, m := range room.members {
		if m != exclude {
			out = append(out, m)
		}
	}
	return out
}

// updateConfig updates the room config (host only) and returns a snapshot of
// the other members to notify. Returns an error if member is not the host.
func (h *Hub) updateConfig(room *Room, member *Member, config string) ([]*Member, error) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if member != room.host {
		return nil, fmt.Errorf("not_host")
	}
	room.config = config
	return othersSnapshot(room, member), nil
}

// ---------------------------------------------------------------------------
// WebSocket upgrader
// ---------------------------------------------------------------------------

var upgrader = websocket.Upgrader{
	CheckOrigin:     func(*http.Request) bool { return true },
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

// ---------------------------------------------------------------------------
// Connection handler
// ---------------------------------------------------------------------------

func (h *Hub) serveWS(w http.ResponseWriter, r *http.Request) {
	raw, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("upgrade error: %v", err)
		return
	}

	conn := &safeConn{ws: raw}
	log.Printf("connected  addr=%s", raw.RemoteAddr())

	// Keepalive: server pings periodically, client must pong within pongWait.
	raw.SetReadDeadline(time.Now().Add(pongWait))
	raw.SetPongHandler(func(string) error {
		return raw.SetReadDeadline(time.Now().Add(pongWait))
	})

	pingTicker := time.NewTicker(pingInterval)
	defer pingTicker.Stop()

	// stopPing is closed by the deferred cleanup below to signal the ping goroutine.
	stopPing := make(chan struct{})
	defer close(stopPing)

	// Ping goroutine – separate goroutine is safe because safeConn serialises writes.
	go func() {
		for {
			select {
			case <-pingTicker.C:
				if err := conn.ping(); err != nil {
					return
				}
			case <-stopPing:
				return
			}
		}
	}()

	// myMember is set once the conn has joined or hosted a room.
	// After that point all writes from this goroutine go through myMember.send()
	// so they participate in the same write mutex as broadcasts from other goroutines.
	var myMember *Member

	defer func() {
		pingTicker.Stop()
		raw.Close()

		res := h.leave(conn)
		if res == nil {
			return
		}
		log.Printf("disconnected  id=%s room=%s closed=%v", res.member.id, res.room.token, res.roomClosed)

		if res.roomClosed {
			for _, m := range res.remaining {
				m.send(OutMsg{Type: "room_closed", Reason: "host_left"})
			}
		} else {
			for _, m := range res.remaining {
				m.send(OutMsg{
					Type:    "member_left",
					From:    res.member.id,
					Members: len(res.remaining),
				})
			}
		}
	}()

	// directWrite is used before we have a member reference (before host/join).
	// After that always use myMember.send() to go through the write mutex.
	directWrite := func(msg OutMsg) { _ = conn.write(msg) }

	for {
		_, raw, err := conn.ws.ReadMessage()
		if err != nil {
			break
		}

		var msg InMsg
		if err := json.Unmarshal(raw, &msg); err != nil {
			if myMember != nil {
				myMember.send(OutMsg{Type: "error", Reason: "invalid_json"})
			} else {
				directWrite(OutMsg{Type: "error", Reason: "invalid_json"})
			}
			continue
		}

		switch msg.Type {

		// ---- host -------------------------------------------------------
		case "host":
			if myMember != nil {
				myMember.send(OutMsg{Type: "error", Reason: "already_in_room"})
				continue
			}
			room, member := h.hostRoom(conn, msg.Config)
			myMember = member
			myMember.send(OutMsg{Type: "token", Token: room.token})
			log.Printf("room created  token=%s host=%s", room.token, member.id)

		// ---- join -------------------------------------------------------
		case "join":
			if myMember != nil {
				myMember.send(OutMsg{Type: "error", Reason: "already_in_room"})
				continue
			}
			room, member, err := h.joinRoom(conn, msg.Token)
			if err != nil {
				directWrite(OutMsg{Type: "rejected", Reason: err.Error()})
				continue
			}
			myMember = member

			// Snapshot room state and other members while holding the lock.
			h.mu.Lock()
			cfg := room.config
			count := len(room.members)
			others := othersSnapshot(room, member)
			h.mu.Unlock()

			// Tell the joiner they're in.
			myMember.send(OutMsg{
				Type:    "joined",
				ID:      member.id,
				Config:  cfg,
				Members: count,
			})
			// Notify everyone else.
			for _, m := range others {
				m.send(OutMsg{
					Type:    "member_joined",
					From:    member.id,
					Members: count,
				})
			}
			log.Printf("room %s  %s joined  (%d/%d)", room.token, member.id, count, room.maxSize)

		// ---- message ----------------------------------------------------
		// Relay an arbitrary JSON payload to all other room members.
		case "message":
			if myMember == nil {
				directWrite(OutMsg{Type: "error", Reason: "not_in_room"})
				continue
			}
			room, member, ok := h.lookup(conn)
			if !ok {
				myMember.send(OutMsg{Type: "error", Reason: "not_in_room"})
				continue
			}
			h.mu.Lock()
			others := othersSnapshot(room, member)
			h.mu.Unlock()

			out := OutMsg{
				Type:    "message",
				From:    member.id,
				Payload: msg.Payload,
			}
			for _, m := range others {
				m.send(out)
			}

		// ---- set_config -------------------------------------------------
		// Host updates the room config; all other members are notified.
		case "set_config":
			if myMember == nil {
				directWrite(OutMsg{Type: "error", Reason: "not_in_room"})
				continue
			}
			room, member, ok := h.lookup(conn)
			if !ok {
				myMember.send(OutMsg{Type: "error", Reason: "not_in_room"})
				continue
			}
			others, err := h.updateConfig(room, member, msg.Config)
			if err != nil {
				myMember.send(OutMsg{Type: "error", Reason: err.Error()})
				continue
			}
			for _, m := range others {
				m.send(OutMsg{Type: "config_updated", Config: msg.Config})
			}

		// ---- disconnect -------------------------------------------------
		case "disconnect":
			return // triggers defer cleanup

		default:
			if myMember != nil {
				myMember.send(OutMsg{Type: "error", Reason: "unknown_type"})
			} else {
				directWrite(OutMsg{Type: "error", Reason: "unknown_type"})
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = defaultPort
	}

	maxRoom := defaultMaxRoomSize
	if v := os.Getenv("MAX_ROOM_SIZE"); v != "" {
		if n, err := strconv.Atoi(v); err == nil && n >= 2 {
			maxRoom = n
		} else {
			log.Printf("invalid MAX_ROOM_SIZE %q, using default %d", v, defaultMaxRoomSize)
		}
	}

	hub := newHub(maxRoom)

	mux := http.NewServeMux()
	mux.HandleFunc("/ws", hub.serveWS)
	mux.HandleFunc("/", func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("OK\n"))
	})

	addr := ":" + port
	log.Printf("socket-server-go  addr=%s  max-room-size=%d", addr, maxRoom)
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatalf("fatal: %v", err)
	}
}
