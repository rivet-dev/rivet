import { actor } from "rivetkit";

interface ConnParams {
  authToken: string;
}

interface ConnInfo {
  userId: string;
  role: string;
}

// Example validation functions
function validateToken(token: string): boolean {
  return token.length > 0;
}

function getUserIdFromToken(token: string): string {
  return "user-" + token.slice(0, 8);
}

const gameRoom = actor({
  // Per-connection data, keyed by connection ID
  createVars: () => ({ players: new Map<string, ConnInfo>() }),

  // Validate the authentication token before accepting the connection
  onBeforeConnect: (c, params: ConnParams) => {
    if (!params.authToken || !validateToken(params.authToken)) {
      throw new Error("Invalid auth token");
    }
  },

  // Record who this connection belongs to
  onConnect: (c, conn) => {
    c.vars.players.set(conn.id, {
      userId: getUserIdFromToken(conn.params.authToken),
      role: "player"
    });
  },

  onDisconnect: (c, conn) => {
    c.vars.players.delete(conn.id);
  },

  actions: {}
});
