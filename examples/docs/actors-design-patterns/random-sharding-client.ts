import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

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

const registry = setup({ use: { rateLimiter } });
const client = createClient<typeof registry>("http://localhost:6420");

// Shard randomly: rateLimiter:shard-0, rateLimiter:shard-1, rateLimiter:shard-2
const shardKey = `shard-${Math.floor(Math.random() * 3)}`;
const limiter = client.rateLimiter.getOrCreate([shardKey]);
const allowed = await limiter.checkLimit("user-123", 100);
