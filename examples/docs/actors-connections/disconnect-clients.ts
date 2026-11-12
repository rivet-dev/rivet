import { actor } from "rivetkit";

const secureRoom = actor({
  // Map each connection ID to its user ID
  createVars: () => ({ userIds: new Map<string, string>() }),

  onConnect: (c, conn) => {
    c.vars.userIds.set(conn.id, "user-" + Math.random().toString(36).slice(2, 11));
  },

  onDisconnect: (c, conn) => {
    c.vars.userIds.delete(conn.id);
  },

  actions: {
    kickUser: (c, targetUserId: string, reason?: string) => {
      // Find the connection to kick by iterating over the Map
      for (const conn of c.conns.values()) {
        if (c.vars.userIds.get(conn.id) === targetUserId) {
          // Disconnect with a reason
          conn.disconnect(reason || "Kicked by admin");
          break;
        }
      }
    }
  }
});
