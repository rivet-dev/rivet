import { actor } from "rivetkit";

interface UserStatus {
  userId: string;
  online: boolean;
  lastSeen: number;
}

const chatRoom = actor({
  // Track users in memory, keyed by connection ID
  createVars: () => ({ users: new Map<string, UserStatus>() }),

  onConnect: (c, conn) => {
    // Add the user to the room's user list
    const userId = "user-" + Math.random().toString(36).slice(2, 11);
    c.vars.users.set(conn.id, {
      userId,
      online: true,
      lastSeen: Date.now()
    });

    // Broadcast that a user joined
    c.broadcast("userJoined", { userId, timestamp: Date.now() });

    console.log(`User ${userId} connected`);
  },

  actions: {}
});
