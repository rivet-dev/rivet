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
    privateMessage: event<{
      from?: string;
      message: string;
      timestamp: number;
    }>()
  },

  createConnState: (c, params: { playerId: string, role?: string }): ConnState => ({
    playerId: params.playerId,
    role: params.role || "player"
  }),

  actions: {
    sendPrivateMessage: (c, targetPlayerId: string, message: string) => {
      // Find the target player's connection
      let targetConn = null;
      for (const conn of c.conns.values()) {
        if (conn.state.playerId === targetPlayerId) {
          targetConn = conn;
          break;
        }
      }

      if (targetConn) {
        targetConn.send('privateMessage', {
          from: c.conn?.state.playerId,
          message,
          timestamp: Date.now()
        });
      } else {
        throw new Error("Player not found or not connected");
      }
    }
  }
});
