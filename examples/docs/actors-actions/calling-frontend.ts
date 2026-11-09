import { createClient } from "rivetkit/client";
import { actor, setup } from "rivetkit";

// Define actor
const counter = actor({
  vars: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.vars.count += amount;
      return c.vars.count;
    }
  }
});

// Create registry
const registry = setup({ use: { counter } });

// Create client
const client = createClient<typeof registry>("http://localhost:6420");
const counterActor = await client.counter.getOrCreate();
const result = await counterActor.increment(42);
console.log(result); // The value returned by the action
