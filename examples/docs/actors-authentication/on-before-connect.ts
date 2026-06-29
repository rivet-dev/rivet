import { actor, UserError } from "rivetkit";

interface ConnParams {
  authToken: string;
}

// Example token validation function
async function validateToken(token: string, roomKey: string[]): Promise<boolean> {
  // In production, verify JWT or call auth service
  return token.length > 0 && roomKey.length > 0;
}

interface Message {
  text: string;
  timestamp: number;
}

const chatRoom = actor({
  state: { messages: [] as Message[] },

  onBeforeConnect: async (c, params: ConnParams) => {
    const roomName = c.key;
    const isValid = await validateToken(params.authToken, roomName);
    if (!isValid) {
      throw new UserError("Forbidden", { code: "forbidden" });
    }
  },

  actions: {
    sendMessage: (c, text: string) => {
      c.state.messages.push({ text, timestamp: Date.now() });
    },
  },
});
