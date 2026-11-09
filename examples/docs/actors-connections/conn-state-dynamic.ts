import { actor } from "rivetkit";

interface ConnInfo {
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
  createVars: () => ({
    messages: [] as Message[],
    // Per-connection data, keyed by connection ID
    connections: new Map<string, ConnInfo>()
  }),

  // Build each connection's data when it connects
  onConnect: (c, conn) => {
    c.vars.connections.set(conn.id, {
      userId: generateUserId(),
      role: "guest",
      joinedAt: Date.now()
    });
  },

  onDisconnect: (c, conn) => {
    c.vars.connections.delete(conn.id);
  },

  actions: {
    sendMessage: (c, message: string) => {
      const connInfo = c.vars.connections.get(c.conn.id);
      if (!connInfo) throw new Error("Unknown connection");
      const username = connInfo.userId;
      c.vars.messages.push({ username, message });
      c.broadcast("newMessage", { username, message });
    }
  }
});
