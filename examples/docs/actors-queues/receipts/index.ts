import { actor, queue, setup } from "rivetkit";

export const counter = actor({
  state: { value: 0 },
  queues: {
    increment: queue<{ amount: number }>({
      retry: { maxAttempts: 5 },
      onMessage: async (c, message, { signal }) => {
        signal.throwIfAborted();
        c.state.value += message.body.amount;
      },
    }),
  },
});

export const registry = setup({ use: { counter } });
