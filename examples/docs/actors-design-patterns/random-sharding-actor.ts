import { actor, setup } from "rivetkit";

const rateLimiter = actor({
  vars: { requests: {} as Record<string, number> },
  actions: {
    checkLimit: (c, userId: string, limit: number) => {
      const count = c.vars.requests[userId] ?? 0;
      if (count >= limit) return false;
      c.vars.requests[userId] = count + 1;
      return true;
    },
  },
});

export const registry = setup({
  use: { rateLimiter },
});
