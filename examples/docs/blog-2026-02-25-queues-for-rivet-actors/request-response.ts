import { actor, queue, setup } from "rivetkit";

const counter = actor({
  state: { value: 0 },
  queues: {
    // Second type parameter is the response type
    increment: queue<{ amount: number }, { value: number }>(),
  },
  run: async (c) => {
    // Enable completable to allow responding to callers
    for await (const message of c.queue.iter({ completable: true })) {
      c.state.value += message.body.amount;
      // Send typed response back to the caller
      await message.complete({ value: c.state.value });
    }
  },
});

export const registry = setup({ use: { counter } });
