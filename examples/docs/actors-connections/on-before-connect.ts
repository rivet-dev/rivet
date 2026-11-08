import { actor } from "rivetkit";

interface ConnParams {
  authToken?: string;
  userId?: string;
  role?: string;
}

interface ConnInfo {
  userId: string;
  role: string;
  joinTime: number;
}

function validateToken(token: string): boolean {
  return token.length > 0;
}

const chatRoom = actor({
  // Per-connection data, keyed by connection ID
  createVars: () => ({ connections: new Map<string, ConnInfo>() }),

  // Validate connections before accepting them
  onBeforeConnect: (c, params: ConnParams) => {
    // Validate authentication
    const authToken = params.authToken;
    if (!authToken || !validateToken(authToken)) {
      throw new Error("Invalid authentication");
    }

    // Authentication is valid, so the connection will proceed to onConnect
  },

  // Record per-connection data once the connection is accepted
  onConnect: (c, conn) => {
    c.vars.connections.set(conn.id, {
      userId: conn.params.userId || "anonymous",
      role: conn.params.role || "guest",
      joinTime: Date.now()
    });
  },

  onDisconnect: (c, conn) => {
    c.vars.connections.delete(conn.id);
  },

  actions: {}
});
