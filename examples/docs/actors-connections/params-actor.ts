import { actor } from "rivetkit";

interface ConnParams {
  authToken: string;
}

interface ConnState {
  userId: string;
  role: string;
}

// Example validation functions
function validateToken(token: string): boolean {
  return token.length > 0;
}

function getUserIdFromToken(token: string): string {
  return "user-" + token.slice(0, 8);
}

const gameRoom = actor({
  state: {},

  // Handle connection setup
  createConnState: (c, params: ConnParams): ConnState => {
    // Validate authentication token
    const authToken = params.authToken;

    if (!authToken || !validateToken(authToken)) {
      throw new Error("Invalid auth token");
    }

    // Create connection state
    return { userId: getUserIdFromToken(authToken), role: "player" };
  },

  actions: {}
});
