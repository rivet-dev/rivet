import { actor, queue, setup } from "rivetkit";

export const worker = actor({
  state: {},
  queues: {
    jobs: queue<{ task: string }>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      const ageMs = Date.now() - message.createdAt;
      if (ageMs > 60_000) {
        // Message is older than 60 seconds, skip it.
        continue;
      }
      console.log("Processing", message.body.task);
    }
  },
});

export const registry = setup({ use: { worker } });
