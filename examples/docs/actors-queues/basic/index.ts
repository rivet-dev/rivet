import { actor, queue, setup } from "rivetkit";

export const counter = actor({
  state: { value: 0 },
  queues: {
    increment: queue<{ amount: number }>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      c.state.value += message.body.amount;
    }
  },
});

export const registry = setup({ use: { counter } });
