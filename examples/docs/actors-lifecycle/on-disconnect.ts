import { actor } from "rivetkit";

const chatRoom = actor({
  createVars: () => ({
    users: {} as Record<string, { online: boolean; lastSeen: number }>,
    userIds: new Map<string, string>(),
  }),

  onConnect: (c, conn) => {
    const params = conn.params as { userId?: string } | undefined;
    c.vars.userIds.set(conn.id, params?.userId ?? "anonymous");
  },

  onDisconnect: (c, conn) => {
    // Update user status when they disconnect
    const userId = c.vars.userIds.get(conn.id) ?? "anonymous";
    c.vars.userIds.delete(conn.id);
    if (c.vars.users[userId]) {
      c.vars.users[userId].online = false;
      c.vars.users[userId].lastSeen = Date.now();
    }

    // Broadcast that a user left
    c.broadcast("userLeft", { userId, timestamp: Date.now() });

    console.log(`User ${userId} disconnected`);
  },

  actions: { /* ... */ }
});
