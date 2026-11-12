import { type ActorContextOf, actor } from "rivetkit";

// Example: Tick loop
const tickActor = actor({
  vars: {
    tickCount: 0,
    // `onWake` stores its loop here so `onSleep` can wait for it.
    loop: null as Promise<void> | null,
  },

  // `onWake` runs on every start, so the loop restarts after a sleep or crash
  onWake: (c) => {
    c.vars.loop = runTickLoop(c).catch((error) => {
      c.log.error({ msg: "tick loop failed", error });
    });
  },

  // `c.abortSignal` fires before `onSleep`, so the loop is already exiting
  onSleep: async (c) => {
    await c.vars.loop;
  },

  actions: {
    getTickCount: (c) => c.vars.tickCount
  }
});

async function runTickLoop(c: ActorContextOf<typeof tickActor>): Promise<void> {
  c.log.info("Background loop started");

  while (!c.aborted) {
    c.vars.tickCount++;
    c.log.info({ msg: "tick", count: c.vars.tickCount });

    await delay(1_000, c.abortSignal);
  }

  c.log.info("Background loop exiting gracefully");
}

// Resolve after `ms`, or as soon as the actor starts shutting down
function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }

    const finish = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", finish);
      resolve();
    };
    const timer = setTimeout(finish, ms);
    signal.addEventListener("abort", finish, { once: true });
  });
}
