import { actor } from "rivetkit";

const chatRoom = actor({
  createVars: () => ({
    users: {} as Record<string, { online: boolean; lastSeen: number }>,
    userIds: new Map<string, string>(),
  }),

  onConnect: (c, conn) => {
    // Remember which user owns this connection
    const params = conn.params as { userId?: string } | undefined;
    const userId = params?.userId ?? "anonymous";
    c.vars.userIds.set(conn.id, userId);

    // Add user to the room's user list
    c.vars.users[userId] = {
      online: true,
      lastSeen: Date.now()
    };

    // Broadcast that a user joined
    c.broadcast("userJoined", { userId, timestamp: Date.now() });

    console.log(`User ${userId} connected`);
  },

  actions: { /* ... */ }
});
