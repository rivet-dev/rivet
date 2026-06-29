import { actor, UserError } from "rivetkit";

const api = actor({
  state: { requestCount: 0, lastReset: Date.now() },
  actions: {
    makeRequest: (c) => {
      c.state.requestCount++;

      const limit = 100;
      if (c.state.requestCount > limit) {
        const resetAt = c.state.lastReset + 60_000; // Reset after 1 minute

        throw new UserError("Rate limit exceeded", {
          code: "rate_limited",
          metadata: {
            limit: limit,
            resetAt: resetAt,
            retryAfter: Math.ceil((resetAt - Date.now()) / 1000)
          }
        });
      }

      // Rest of request logic...
    }
  }
});
