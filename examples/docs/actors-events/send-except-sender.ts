import { actor, event } from "rivetkit";

interface ConnState {
  playerId: string;
  role: string;
}

const gameRoom = actor({
  state: {
    players: {} as Record<string, {health: number, position: {x: number, y: number}}>
  },

  events: {
    playerMoved: event<{
      playerId: string;
      position: { x: number; y: number };
    }>()
  },

  createConnState: (c, params: { playerId: string, role?: string }): ConnState => ({
    playerId: params.playerId,
    role: params.role || "player"
  }),

  actions: {
    updatePlayerPosition: (c, position: {x: number, y: number}) => {
      const playerId = c.conn?.state.playerId;
      if (!playerId) return;

      if (c.state.players[playerId]) {
        c.state.players[playerId].position = position;

        // Send position update to all OTHER players
        for (const conn of c.conns.values()) {
          if (conn.state.playerId !== playerId) {
            conn.send('playerMoved', { playerId, position });
          }
        }
      }
    }
  }
});
