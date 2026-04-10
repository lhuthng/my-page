# Socket Server

Node.js WebSocket server that pairs two browser clients for the L-game feature — one hosts a session, the other joins using a short token, and the server relays game commands between them.

- **Port:** 5001
- **Live:** `wss://ws.huuthangle.site`

## Protocol

All messages are plain text. The server is stateless beyond active connections.

### Hosting a session

```
→ host:<config>
← token:472
```

Registers the sender as a host with a game config. The server assigns a random 3-digit token unique among active sessions.

### Joining a session

```
→ join:<token>

← paired:<config>    (joiner — success, receives host's config)
← paired:            (host — notified that someone joined)
← rejected:          (joiner — token not found)
```

### Relaying commands

Once paired, either side can send commands:

```
→ command:<type>:<data>
```

The server forwards `<type>:<data>` to the other client. If the sender isn't paired, they receive `unpaired:` instead.

### Updating config (host only)

```
→ command:set_turn:<data>
```

Updates the stored config for the session. Useful so late-joining clients or reconnects can receive the current state.

### Disconnecting

Either side can send `disconnect` or simply close the connection. The server unpairs both sides and sends `unpaired:` to whichever client is still connected.

## Local Development

```bash
node server.js
# Listening on ws://localhost:5001
```

## Deployment

Hosted on [Fly.io](https://fly.io). To deploy manually:

```bash
# from socket-server/
flyctl deploy --remote-only
```

Or just push changes to `socket-server/**` on `master` — the `deploy-socket-server` GitHub Actions job handles it automatically.