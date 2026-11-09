import { actor } from "rivetkit";

function validateToken(token: string): boolean {
  return token.length > 0;
}

type ConnParams = {
  userId?: string;
  role?: string;
  authToken?: string;
};

interface ConnInfo {
  userId: string;
  role: string;
  joinTime: number;
}

const chatRoom = actor({
  // Per-connection data lives in memory, keyed by connection ID
  createVars: () => ({ connections: new Map<string, ConnInfo>() }),

  // Validate connections before accepting them
  onBeforeConnect: (_c, params: ConnParams) => {
    // Validate authentication
    const authToken = params.authToken;
    if (!authToken || !validateToken(authToken)) {
      throw new Error("Invalid authentication");
    }

    // Authentication is valid, so the connection will proceed to onConnect
  },

  // Record per-connection data once the connection is accepted
  onConnect: (c, conn) => {
    const params = conn.params;
    c.vars.connections.set(conn.id, {
      userId: params.userId || "anonymous",
      role: params.role || "guest",
      joinTime: Date.now()
    });
  },

  onDisconnect: (c, conn) => {
    c.vars.connections.delete(conn.id);
  },

  actions: { /* ... */ }
});
