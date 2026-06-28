import { actor, queue, setup } from "rivetkit";

export const counter = actor({
  state: { value: 0 },
  queues: {
    mutate: queue<{ delta: number }>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      c.state.value += message.body.delta;
    }
  },
  actions: {
    increment: async (c, delta: number) => {
      await c.queue.send("mutate", { delta });
    },
  },
});

export const registry = setup({ use: { counter } });
