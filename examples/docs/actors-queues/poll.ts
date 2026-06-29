import { actor, queue, setup } from "rivetkit";

export const queueWorker = actor({
  state: {},
  queues: {
    jobs: queue<{ id: string }>(),
  },
  actions: {
    poll: async (c) => {
      const immediate = await c.queue.tryNext();

      const immediateBatch = await c.queue.tryNextBatch({
        count: 10,
      });

      return {
        hasImmediate: immediate !== undefined,
        immediateBatchCount: immediateBatch.length,
      };
    },
  },
});

export const registry = setup({ use: { queueWorker } });
