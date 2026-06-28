import { actor, queue, setup } from "rivetkit";

export const counter = actor({
  state: { value: 0 },
  queues: {
    increment: queue<{ amount: number }, undefined>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter({ completable: true })) {
      c.state.value += message.body.amount;
      await message.complete();
    }
  },
});

export const registry = setup({ use: { counter } });
