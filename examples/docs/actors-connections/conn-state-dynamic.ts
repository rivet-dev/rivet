import { actor } from "rivetkit";

interface ConnState {
  userId: string;
  role: string;
  joinedAt: number;
}

interface Message {
  username: string;
  message: string;
}

function generateUserId(): string {
  return "user-" + Math.random().toString(36).slice(2, 11);
}

const chatRoom = actor({
  state: { messages: [] as Message[] },

  // Create connection state dynamically
  createConnState: (c): ConnState => {
    // Return the connection state
    return {
      userId: generateUserId(),
      role: "guest",
      joinedAt: Date.now()
    };
  },

  actions: {
    sendMessage: (c, message: string) => {
      const username = c.conn.state.userId;
      c.state.messages.push({ username, message });
      c.broadcast("newMessage", { username, message });
    }
  }
});
