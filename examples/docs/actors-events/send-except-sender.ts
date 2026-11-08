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
    playerMoved: event<{
      playerId: string;
      position: { x: number; y: number };
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
    updatePlayerPosition: (c, position: {x: number, y: number}) => {
      const playerId = c.vars.playerConns.get(c.conn.id)?.playerId;
      if (!playerId) return;

      if (c.vars.players[playerId]) {
        c.vars.players[playerId].position = position;

        // Send position update to all OTHER players
        for (const conn of c.conns.values()) {
          if (c.vars.playerConns.get(conn.id)?.playerId !== playerId) {
            conn.send('playerMoved', { playerId, position });
          }
        }
      }
    }
  }
});
