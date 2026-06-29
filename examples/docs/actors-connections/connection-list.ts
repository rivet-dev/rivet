import { actor } from "rivetkit";

interface ConnState {
  userId: string;
}

const chatRoom = actor({
  state: { users: {} as Record<string, { online: boolean }> },

  createConnState: (): ConnState => ({
    userId: "user-" + Math.random().toString(36).slice(2, 11)
  }),

  actions: {
    sendDirectMessage: (c, recipientId: string, message: string) => {
      // Find the recipient's connection by iterating over the Map
      let recipientConn = null;
      for (const conn of c.conns.values()) {
        if (conn.state.userId === recipientId) {
          recipientConn = conn;
          break;
        }
      }

      if (recipientConn) {
        // Send a private message to just that client
        recipientConn.send("directMessage", {
          from: c.conn.state.userId,
          message: message
        });
      }
    }
  }
});
