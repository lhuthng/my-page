# socket-server

Go WebSocket relay server — rooms up to 4 members, generic JSON message relay, and a clean JSON protocol.

- **Port:** 5001 (configurable via `PORT` env var)
- **Live:** `wss://wss.huuthangle.site/ws`

---

## Protocol

All messages are JSON objects. Every message has a `"type"` field.

### Hosting a room

```json
→ { "type": "host", "config": "<any string>" }
← { "type": "token", "token": "472" }
```

Registers the sender as the host of a new room. The server assigns a random 3-digit token unique among active rooms. The `config` value is stored and sent to every client that joins.

---

### Joining a room

```json
→ { "type": "join", "token": "472" }
```

**Success** (joiner receives):
```json
← { "type": "joined", "id": "a1b2c3d4", "config": "<host config>", "members": 2 }
```

**Everyone else in the room receives:**
```json
← { "type": "member_joined", "from": "a1b2c3d4", "members": 2 }
```

**Failure:**
```json
← { "type": "rejected", "reason": "not_found" }
← { "type": "rejected", "reason": "room_full" }
```

`id` is the caller's assigned member ID, used to identify senders of relayed messages.

---

### Relaying a message

Once in a room, any member can relay an arbitrary JSON payload to every other member:

```json
→ { "type": "message", "payload": <any JSON value> }
```

All other room members receive:
```json
← { "type": "message", "from": "<sender id>", "payload": <any JSON value> }
```

`payload` can be any valid JSON — a string, number, object, or array. The server forwards it verbatim.

**Examples:**

```json
→ { "type": "message", "payload": "hello" }
→ { "type": "message", "payload": { "action": "move", "x": 3, "y": 1 } }
→ { "type": "message", "payload": [1, 2, 3] }
```

---

### Updating room config (host only)

```json
→ { "type": "set_config", "config": "<new config>" }
```

Updates the config stored for the room. All other members are notified:

```json
← { "type": "config_updated", "config": "<new config>" }
```

Returns an error if sent by a non-host member.

---

### Disconnecting

```json
→ { "type": "disconnect" }
```

Or simply close the WebSocket connection. The server cleans up automatically.

**If a non-host member leaves**, all remaining members receive:
```json
← { "type": "member_left", "from": "<id>", "members": 2 }
```

**If the host leaves**, the room is destroyed and all remaining members receive:
```json
← { "type": "room_closed", "reason": "host_left" }
```

---

### Error responses

```json
← { "type": "error", "reason": "already_in_room" }
← { "type": "error", "reason": "not_in_room" }
← { "type": "error", "reason": "not_host" }
← { "type": "error", "reason": "invalid_json" }
← { "type": "error", "reason": "unknown_type" }
```

---

## Message type reference

| Direction | `type`          | Description                                      |
|-----------|-----------------|--------------------------------------------------|
| → server  | `host`          | Create a room, receive a token                   |
| → server  | `join`          | Join a room by token                             |
| → server  | `message`       | Relay a payload to all other room members        |
| → server  | `set_config`    | Update room config (host only)                   |
| → server  | `disconnect`    | Gracefully close the connection                  |
| ← client  | `token`         | Assigned token after hosting                     |
| ← client  | `joined`        | Confirmation + room state after joining          |
| ← client  | `member_joined` | Another member joined your room                  |
| ← client  | `message`       | Relayed message from another member              |
| ← client  | `config_updated`| Host updated the room config                     |
| ← client  | `member_left`   | A non-host member disconnected                   |
| ← client  | `room_closed`   | Host disconnected; room is gone                  |
| ← client  | `rejected`      | Join failed (not found / room full)              |
| ← client  | `error`         | Protocol or permission error                     |

---

## Configuration

| Env var         | Default | Description                          |
|-----------------|---------|--------------------------------------|
| `PORT`          | `5001`  | Port to listen on                    |
| `MAX_ROOM_SIZE` | `4`     | Maximum members per room (min 2)     |

---

## Local development

```bash
go run .
# or
go build -o server . && ./server

# custom port / room size
PORT=8080 MAX_ROOM_SIZE=2 go run .
```

---

## Deployment

Hosted on an Oracle Cloud VM (`130.61.121.230`) behind nginx + Let's Encrypt TLS.

Push changes to `socket-server/**` on `master` — the `deploy-socket-server` GitHub Actions job will:
1. Cross-compile a `linux/amd64` binary on GitHub's servers
2. SCP it to the VM
3. SSH in and restart the `socket-server` systemd service

To deploy manually:

```bash
# cross-compile
GOOS=linux GOARCH=amd64 go build -ldflags="-s -w" -o server-linux .

# copy and restart
scp -i ~/.ssh/ssh-key-support-server.key server-linux ubuntu@130.61.121.230:/tmp/socket-server
ssh -i ~/.ssh/ssh-key-support-server.key ubuntu@130.61.121.230 "
  sudo mv /tmp/socket-server /usr/local/bin/socket-server
  sudo chmod +x /usr/local/bin/socket-server
  sudo systemctl restart socket-server
"
```

Required GitHub secrets: `WS_VM_HOST`, `WS_VM_USER`, `WS_VM_SSH_KEY`.
See [`.github/README.md`](../.github/README.md) for full infrastructure details.

---

## Differences from the original Node.js server

| Feature | Node.js server | Go server |
|---|---|---|
| Message format | Plain text (`type:data`) | JSON |
| Max room size | 2 (one host + one guest) | 4 (configurable) |
| Message relay | `command:<type>:<data>` only | Generic `payload` (any JSON) |
| Config update | `command:set_turn:<data>` | `set_config` message |
| Member IDs | None | Each member gets a random ID |
| Connection health | None | Ping/pong keepalive (30 s) |
| Concurrency | Single-threaded (Node.js) | Goroutines + mutex-guarded state |