const { createServer } = require("http");
const { WebSocketServer } = require("ws");
const crypto = require("crypto");

const server = createServer().listen(5001, () => {
  console.log("5001");
});
const wss = new WebSocketServer({ server });

// host socket -> token
const hosts = new Map();

// token -> { socket, config }
const tokens = new Map();

// socket -> paired socket
const pairs = new Map();

function generateToken() {
  let token;
  do {
    token = crypto.randomInt(100, 1000).toString();
  } while (tokens.has(token));
  return token;
}

wss.on("connection", (ws) => {
  ws.on("error", console.error);

  ws.on("message", (raw) => {
    const msg = raw.toString();

    // host:<config>
    if (msg.startsWith("host:")) {
      const config = msg.substring(5);
      const token = generateToken();

      tokens.set(token, { socket: ws, config });
      hosts.set(ws, token);

      ws.send(`token:${token}`);
      return;
    }

    // join:<token>
    if (msg.startsWith("join:")) {
      const token = msg.substring(5);

      if (tokens.has(token)) {
        const hostSocket = tokens.get(token).socket;

        ws.send(`paired:${tokens.get(token).config}`);
        pairs.set(ws, hostSocket);
        pairs.set(hostSocket, ws);

        hostSocket.send("paired:");
      } else {
        ws.send("rejected:");
      }

      return;
    }

    // command:type:data
    if (msg.startsWith("command:")) {
      const [, type, data] = msg.split(":");

      if (pairs.has(ws)) {
        const other = pairs.get(ws);
        other.send(`${type}:${data}`);
        return;
      }

      // host updates its config
      if (type === "set_turn" && hosts.has(ws)) {
        const token = hosts.get(ws);
        tokens.get(token).config = data;
        return;
      }

      ws.send("unpaired:");
      return;
    }

    // disconnect (optional handling)
    if (msg.startsWith("disconnect")) {
      ws.close();
    }
  });

  ws.on("close", () => {
    // unpair if paired
    if (pairs.has(ws)) {
      const other = pairs.get(ws);
      if (pairs.has(other)) {
        other.send("unpaired:");
        pairs.delete(other);
      }
      pairs.delete(ws);
    }

    // remove host + token if it was host
    if (hosts.has(ws)) {
      const token = hosts.get(ws);
      tokens.delete(token);
      hosts.delete(ws);
    }
  });
});
