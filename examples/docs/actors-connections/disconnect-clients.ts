import { actor } from "rivetkit";

interface ConnState {
  userId: string;
}

const secureRoom = actor({
  state: {},

  createConnState: (): ConnState => ({
    userId: "user-" + Math.random().toString(36).slice(2, 11)
  }),

  actions: {
    kickUser: (c, targetUserId: string, reason?: string) => {
      // Find the connection to kick by iterating over the Map
      for (const conn of c.conns.values()) {
        if (conn.state.userId === targetUserId) {
          // Disconnect with a reason
          conn.disconnect(reason || "Kicked by admin");
          break;
        }
      }
    }
  }
});
