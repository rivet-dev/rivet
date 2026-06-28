import { actor } from "rivetkit";

// Example: Tick loop
const tickActor = actor({
  state: { tickCount: 0 },

  run: async (c) => {
    c.log.info("Background loop started");

    while (!c.aborted) {
      c.state.tickCount++;
      c.log.info({ msg: "tick", count: c.state.tickCount });

      // Wait 1 second. Final shutdown also resolves this wait.
      await new Promise<void>((resolve) => {
        const timeout = setTimeout(resolve, 1000);
        c.abortSignal.addEventListener("abort", () => {
          clearTimeout(timeout);
          resolve();
        }, { once: true });
      });
    }

    c.log.info("Background loop exiting gracefully");
  },

  actions: {
    getTickCount: (c) => c.state.tickCount
  }
});
