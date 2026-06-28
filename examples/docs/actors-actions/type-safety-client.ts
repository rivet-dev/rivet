import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

// Define the actor inline for type inference
const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, count: number) => {
      c.state.count += count;
      return c.state.count;
    }
  }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>("http://localhost:6420");

// Type-safe client usage
const counterActor = await client.counter.get();
await counterActor.increment(123); // OK
// await counterActor.increment("non-number type"); // TypeScript error
// await counterActor.nonexistentMethod(123); // TypeScript error
