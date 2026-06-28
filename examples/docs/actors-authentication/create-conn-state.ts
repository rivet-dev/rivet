import { actor, UserError } from "rivetkit";

interface ConnParams {
  authToken: string;
}

interface ConnState {
  userId: string;
  role: string;
}

interface Message {
  userId: string;
  text: string;
  timestamp: number;
}

// Example token validation function
async function validateToken(token: string, roomKey: string[]): Promise<{ sub: string; role: string } | null> {
  // In production, verify JWT or call auth service
  if (token.length > 0 && roomKey.length > 0) {
    return { sub: "user-123", role: "member" };
  }
  return null;
}

const chatRoom = actor({
  state: { messages: [] as Message[] },

  createConnState: async (c, params: ConnParams): Promise<ConnState> => {
    const roomName = c.key;
    const payload = await validateToken(params.authToken, roomName);
    if (!payload) {
      throw new UserError("Forbidden", { code: "forbidden" });
    }
    return {
      userId: payload.sub,
      role: payload.role,
    };
  },

  actions: {
    sendMessage: (c, text: string) => {
      // Access user data via c.conn.state
      const { userId, role } = c.conn.state;

      if (role !== "member") {
        throw new UserError("Insufficient permissions", { code: "insufficient_permissions" });
      }

      c.state.messages.push({ userId, text, timestamp: Date.now() });
      c.broadcast("newMessage", { userId, text });
    },
  },
});
