import { actor } from "rivetkit";

const chatRoom = actor({
  state: {
    users: {} as Record<string, { online: boolean; lastSeen: number }>,
    messages: [] as string[],
  },

  createConnState: (_c, params: { userId?: string }) => ({
    userId: params.userId ?? "anonymous",
  }),

  onDisconnect: (c, conn) => {
    // Update user status when they disconnect
    const userId = conn.state.userId;
    if (c.state.users[userId]) {
      c.state.users[userId].online = false;
      c.state.users[userId].lastSeen = Date.now();
    }

    // Broadcast that a user left
    c.broadcast("userLeft", { userId, timestamp: Date.now() });

    console.log(`User ${userId} disconnected`);
  },

  actions: { /* ... */ }
});
