import { actor, queue, setup } from "rivetkit";

const counter = actor({
  state: { value: 0 },
  // Define typed queues
  queues: {
    increment: queue<{ amount: number }>(),
  },
  // Process messages in the run handler
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      c.state.value += message.body.amount;
    }
  },
  actions: {
    getValue: (c) => c.state.value,
  },
});

export const registry = setup({ use: { counter } });
