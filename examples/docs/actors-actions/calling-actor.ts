import { actor, setup } from "rivetkit";

// Define counter actor
const counter = actor({
  vars: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.vars.count += amount;
      return c.vars.count;
    }
  }
});

// Define actorA that calls counter
const actorA = actor({
  actions: {
    callOtherActor: async (c) => {
      const client = c.client();
      const counterActor = await client.counter.getOrCreate();
      return await counterActor.increment(10);
    }
  }
});

// Create registry
export const registry = setup({ use: { counter, actorA } });
