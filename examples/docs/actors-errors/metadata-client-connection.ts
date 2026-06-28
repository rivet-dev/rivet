import { actor, setup } from "rivetkit";
import { createClient, ActorError } from "rivetkit/client";

const api = actor({
  state: { requestCount: 0 },
  actions: { makeRequest: (c) => {} }
});

const registry = setup({ use: { api } });
const client = createClient<typeof registry>("http://localhost:6420");
const conn = client.api.getOrCreate([]).connect();

try {
  await conn.makeRequest();
} catch (error) {
  if (error instanceof ActorError) {
    console.log(error.message); // "Rate limit exceeded"
    console.log(error.code); // "rate_limited"
    console.log(error.metadata); // { limit: 100, resetAt: 1234567890, retryAfter: 45 }

    if (error.code === "rate_limited") {
      const metadata = error.metadata as { retryAfter: number };
      console.log(`Rate limit hit. Try again in ${metadata.retryAfter} seconds`);
    }
  }
}
