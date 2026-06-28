import { actor } from "rivetkit";

interface ConnState {
  userId: string;
}

interface UserStatus {
  online: boolean;
  lastSeen: number;
}

const chatRoom = actor({
  state: { users: {} as Record<string, UserStatus>, messages: [] as string[] },

  createConnState: (): ConnState => ({
    userId: "user-" + Math.random().toString(36).slice(2, 11)
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

  actions: {}
});
