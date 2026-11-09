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
    const userId = "user-" + Math.random().toString(36).slice(2, 11);
    c.vars.users.set(conn.id, { userId, online: true, lastSeen: Date.now() });
  },

  onDisconnect: (c, conn) => {
    // Clean up the user's entry when they disconnect
    const user = c.vars.users.get(conn.id);
    if (!user) return;
    c.vars.users.delete(conn.id);

    // Broadcast that a user left
    c.broadcast("userLeft", { userId: user.userId, timestamp: Date.now() });

    console.log(`User ${user.userId} disconnected`);
  },

  actions: {}
});
