import { actor, event } from "rivetkit";

interface PlayerConn {
  playerId: string;
  role: string;
}

const gameRoom = actor({
  createVars: () => ({
    players: {} as Record<string, {health: number, position: {x: number, y: number}}>,
    // Player info for each live connection, keyed by connection ID
    playerConns: new Map<string, PlayerConn>()
  }),

  events: {
    privateMessage: event<{
      from?: string;
      message: string;
      timestamp: number;
    }>()
  },

  onBeforeConnect: (c, params: { playerId: string, role?: string }) => {
    if (!params.playerId) throw new Error("Missing playerId");
  },

  onConnect: (c, conn) => {
    c.vars.playerConns.set(conn.id, {
      playerId: conn.params.playerId,
      role: conn.params.role || "player"
    });
  },

  onDisconnect: (c, conn) => {
    c.vars.playerConns.delete(conn.id);
  },

  actions: {
    sendPrivateMessage: (c, targetPlayerId: string, message: string) => {
      // Find the target player's connection
      let targetConn = null;
      for (const conn of c.conns.values()) {
        if (c.vars.playerConns.get(conn.id)?.playerId === targetPlayerId) {
          targetConn = conn;
          break;
        }
      }

      if (targetConn) {
        targetConn.send('privateMessage', {
          from: c.vars.playerConns.get(c.conn.id)?.playerId,
          message,
          timestamp: Date.now()
        });
      } else {
        throw new Error("Player not found or not connected");
      }
    }
  }
});
