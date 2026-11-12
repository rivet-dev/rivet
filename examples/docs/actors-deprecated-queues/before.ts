import { actor, queue } from "rivetkit";

// An actor that still uses the deprecated `queues` option and `c.queue`
const counter = actor({
  queues: {
    increment: queue<{ amount: number }>(),
  },
  createVars: () => ({ value: 0 }),
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      c.vars.value += message.body.amount;
    }
  },
});

// Client: await handle.send("increment", { amount: 5 });
