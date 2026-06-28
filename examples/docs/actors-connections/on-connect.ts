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

  actions: {}
});
