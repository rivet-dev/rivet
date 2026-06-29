import { actor, setup } from "rivetkit";

const rateLimiter = actor({
  state: { requests: {} as Record<string, number> },
  actions: {
    checkLimit: (c, userId: string, limit: number) => {
      const count = c.state.requests[userId] ?? 0;
      if (count >= limit) return false;
      c.state.requests[userId] = count + 1;
      return true;
    },
  },
});

export const registry = setup({
  use: { rateLimiter },
});
