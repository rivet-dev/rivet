import { actor, queue, setup } from "rivetkit";
import { joinSignals } from "rivetkit/utils";

const worker = actor({
  state: {},
  createVars: () => ({
    cancelController: new AbortController(),
  }),
  queues: {
    jobs: queue<{ id: string }>(),
  },
  actions: {
    // Cancel processing from outside
    cancelProcessing: async (c) => {
      c.vars.cancelController.abort();
    },
  },
  run: async (c) => {
    while (!c.aborted) {
      // Combine actor shutdown signal with custom cancel signal
      const signal = joinSignals(
        c.abortSignal,
        c.vars.cancelController.signal,
      );

      try {
        const message = await c.queue.next({ signal });
        if (!message) continue;
        console.log("Processing job", message.body.id);
      } catch (error) {
        // Reset cancel controller if it was a manual cancel
        if (c.vars.cancelController.signal.aborted && !c.aborted) {
          c.vars.cancelController = new AbortController();
          continue;
        }
        throw error;
      }
    }
  },
});

export const registry = setup({ use: { worker } });
