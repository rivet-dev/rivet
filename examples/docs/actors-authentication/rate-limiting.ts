import { actor, UserError } from "rivetkit";

interface ConnParams {
  authToken: string;
}

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

// Example token validation function
async function validateToken(token: string): Promise<{ userId: string }> {
  // In production, verify JWT or call auth service
  return { userId: "user-123" };
}

const rateLimitedActor = actor({
  state: {},
  createVars: () => ({ rateLimits: {} as Record<string, RateLimitEntry> }),

  onBeforeConnect: async (c, params: ConnParams) => {
    // Extract user ID
    const { userId } = await validateToken(params.authToken);

    // Check rate limit
    const now = Date.now();
    const limit = c.vars.rateLimits[userId];

    if (limit && limit.resetAt > now && limit.count >= 10) {
      throw new UserError("Too many requests, try again later", { code: "rate_limited" });
    }

    // Update rate limit
    if (!limit || limit.resetAt <= now) {
      c.vars.rateLimits[userId] = { count: 1, resetAt: now + 60_000 };
    } else {
      limit.count++;
    }
  },

  actions: {
    getData: (c) => ({ success: true }),
  },
});
