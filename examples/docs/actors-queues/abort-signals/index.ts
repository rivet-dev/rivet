import { actor, queue, setup } from "rivetkit";
import { joinSignals } from "rivetkit/utils";

export const signalWorker = actor({
  state: {},
  createVars: () => ({
    cancelController: new AbortController(),
  }),
  queues: {
    jobs: queue<{ id: string }>(),
  },
  actions: {
    cancelProcessing: async (c) => {
      c.vars.cancelController.abort();
    },
  },
  run: async (c) => {
    while (!c.aborted) {
      const signal = joinSignals(c.abortSignal, c.vars.cancelController.signal);

      try {
        const message = await c.queue.next({ signal });
        if (!message) continue;
        console.log("Processing job", message.body.id);
      } catch (error) {
        if (c.vars.cancelController.signal.aborted && !c.aborted) {
          c.vars.cancelController = new AbortController();
          continue;
        }
        throw error;
      }
    }
  },
});

export const registry = setup({ use: { signalWorker } });
