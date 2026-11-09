import { actor } from "rivetkit";

const chatRoom = actor({
  // Map each connection ID to its user ID
  createVars: () => ({ userIds: new Map<string, string>() }),

  onConnect: (c, conn) => {
    c.vars.userIds.set(conn.id, "user-" + Math.random().toString(36).slice(2, 11));
  },

  onDisconnect: (c, conn) => {
    c.vars.userIds.delete(conn.id);
  },

  actions: {
    sendDirectMessage: (c, recipientId: string, message: string) => {
      // Find the recipient's connection by iterating over the Map
      let recipientConn = null;
      for (const conn of c.conns.values()) {
        if (c.vars.userIds.get(conn.id) === recipientId) {
          recipientConn = conn;
          break;
        }
      }

      if (recipientConn) {
        // Send a private message to just that client
        recipientConn.send("directMessage", {
          from: c.vars.userIds.get(c.conn.id),
          message: message
        });
      }
    }
  }
});
