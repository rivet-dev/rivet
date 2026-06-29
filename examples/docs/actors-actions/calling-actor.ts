import { actor, setup } from "rivetkit";

// Define counter actor
const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      return c.state.count;
    }
  }
});

// Define actorA that calls counter
const actorA = actor({
  state: {},
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
