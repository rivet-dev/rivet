import { actor } from "rivetkit";

const chatRoom = actor({
  state: {
    users: {} as Record<string, { online: boolean; lastSeen: number }>,
    messages: [] as string[],
  },

  createConnState: (_c, params: { userId?: string }) => ({
    userId: params.userId ?? "anonymous",
  }),

  onConnect: (c, conn) => {
    // Add user to the room's user list using connection state
    const userId = conn.state.userId;
    c.state.users[userId] = {
      online: true,
      lastSeen: Date.now()
    };

    // Broadcast that a user joined
    c.broadcast("userJoined", { userId, timestamp: Date.now() });

    console.log(`User ${userId} connected`);
  },

  actions: { /* ... */ }
});
