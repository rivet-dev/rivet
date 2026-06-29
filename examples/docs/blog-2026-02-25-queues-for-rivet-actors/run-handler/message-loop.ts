import { actor, queue, setup } from "rivetkit";

const worker = actor({
  state: { processed: 0 },
  queues: {
    jobs: queue<{ url: string }>(),
  },
  // Iterate messages as they arrive
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      await fetch(message.body.url, { method: "POST" });
      c.state.processed += 1;
    }
  },
});

export const registry = setup({ use: { worker } });
