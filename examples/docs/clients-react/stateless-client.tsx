// Stateless: use createClient for one-off calls (SSR or utilities)
import { createClient } from "rivetkit/client";

const client = createClient();
await client.counter.getOrCreate(["my-counter"]).increment(1);
