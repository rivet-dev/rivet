import { actor, UserError } from "rivetkit";

interface ConnParams {
  authToken: string;
}

interface ConnState {
  userId: string;
  role: string;
}

interface TokenCache {
  [token: string]: {
    userId: string;
    role: string;
    expiresAt: number;
  };
}

// Example token validation function
async function validateToken(token: string): Promise<{ sub: string; role: string } | null> {
  // In production, verify JWT or call auth service
  if (token.length > 0) {
    return { sub: "user-123", role: "member" };
  }
  return null;
}

const cachedAuthActor = actor({
  state: {},
  createVars: () => ({ tokenCache: {} as TokenCache }),

  createConnState: async (c, params: ConnParams): Promise<ConnState> => {
    const token = params.authToken;

    // Check cache first
    const cached = c.vars.tokenCache[token];
    if (cached && cached.expiresAt > Date.now()) {
      return { userId: cached.userId, role: cached.role };
    }

    // Validate token (expensive operation)
    const payload = await validateToken(token);
    if (!payload) {
      throw new UserError("Invalid token", { code: "invalid_token" });
    }

    // Cache the result
    c.vars.tokenCache[token] = {
      userId: payload.sub,
      role: payload.role,
      expiresAt: Date.now() + 5 * 60 * 1000, // 5 minutes
    };

    return { userId: payload.sub, role: payload.role };
  },

  actions: {
    getData: (c) => ({ userId: c.conn.state.userId }),
  },
});
