import { actor } from "rivetkit";

const realtimeActor = actor({
  vars: { connectionCount: 0 },

  onWebSocket: (c, websocket) => {
    c.vars.connectionCount++;

    // Send welcome message
    websocket.send(JSON.stringify({
      type: "welcome",
      connectionCount: c.vars.connectionCount
    }));

    // Handle incoming messages
    websocket.addEventListener("message", (event) => {
      const data = JSON.parse(event.data);

      if (data.type === "ping") {
        websocket.send(JSON.stringify({
          type: "pong",
          timestamp: Date.now()
        }));
      }
    });

    // Handle connection close
    websocket.addEventListener("close", () => {
      c.vars.connectionCount--;
    });
  },

  actions: { /* ... */ }
});
