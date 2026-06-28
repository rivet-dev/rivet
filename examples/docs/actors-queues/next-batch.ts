import { actor, queue, setup } from "rivetkit";

export const queueWorker = actor({
  state: {},
  queues: {
    jobs: queue<{ id: string }>(),
  },
  actions: {
    pull: async (c) => {
      const batch = await c.queue.nextBatch({
        count: 10,
        timeout: 1_000,
      });

      const oneWithoutTimeout = await c.queue.next();

      return {
        batchCount: batch.length,
        receivedOneWithoutTimeout: oneWithoutTimeout !== undefined,
      };
    },
  },
});

export const registry = setup({ use: { queueWorker } });
