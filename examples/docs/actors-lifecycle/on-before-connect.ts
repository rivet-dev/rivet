import { actor } from "rivetkit";

function validateToken(token: string): boolean {
  return token.length > 0;
}

type ConnParams = {
  userId?: string;
  role?: string;
  authToken?: string;
};

const chatRoom = actor({
  state: { messages: [] },

  // Method 2: Dynamically create connection state
  createConnState: (_c, params: ConnParams) => {
    return {
      userId: params.userId || "anonymous",
      role: params.role || "guest",
      joinTime: Date.now()
    };
  },

  // Validate connections before accepting them
  onBeforeConnect: (_c, params: ConnParams) => {
    // Validate authentication
    const authToken = params.authToken;
    if (!authToken || !validateToken(authToken)) {
      throw new Error("Invalid authentication");
    }

    // Authentication is valid, connection will proceed
    // The actual connection state will come from connState or createConnState
  },

  actions: { /* ... */ }
});
