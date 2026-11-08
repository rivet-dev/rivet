import { actor } from "rivetkit";

interface ConnInfo {
  role: string;
  joinedAt: number;
}

const chatRoom = actor({
  // Per-connection data, keyed by connection ID
  createVars: () => ({ connections: new Map<string, ConnInfo>() }),

  onConnect: (c, conn) => {
    // Record the join timestamp when a client connects
    c.vars.connections.set(conn.id, { role: "guest", joinedAt: Date.now() });
  },

  onDisconnect: (c, conn) => {
    c.vars.connections.delete(conn.id);
  },

  actions: {
    // ...
  }
});
