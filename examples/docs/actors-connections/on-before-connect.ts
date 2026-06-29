import { actor } from "rivetkit";

interface Message {
  text: string;
  author: string;
}

interface ConnParams {
  authToken?: string;
  userId?: string;
  role?: string;
}

interface ConnState {
  userId: string;
  role: string;
  joinTime: number;
}

function validateToken(token: string): boolean {
  return token.length > 0;
}

const chatRoom = actor({
  state: { messages: [] as Message[] },

  // Dynamically create connection state
  createConnState: (c, params: ConnParams): ConnState => {
    return {
      userId: params.userId || "anonymous",
      role: params.role || "guest",
      joinTime: Date.now()
    };
  },

  // Validate connections before accepting them
  onBeforeConnect: (c, params: ConnParams) => {
    // Validate authentication
    const authToken = params.authToken;
    if (!authToken || !validateToken(authToken)) {
      throw new Error("Invalid authentication");
    }

    // Authentication is valid, connection will proceed
    // The actual connection state will come from createConnState
  },

  actions: {}
});
