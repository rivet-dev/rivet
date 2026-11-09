import { actor } from "rivetkit";

interface PollerVars {
  lastPrice: number;
  // `onWake` stores its loop here so `onSleep` can wait for it.
  task: Promise<void> | null;
}

interface PollerContext {
  vars: PollerVars;
  aborted: boolean;
  abortSignal: AbortSignal;
  keepAwake<T>(promise: Promise<T>): Promise<T>;
  log: { error(fields: Record<string, unknown>): void };
}

async function pollPrices(c: PollerContext): Promise<void> {
  while (!c.aborted) {
    // Keep the actor awake only while a fetch is in flight. Between polls
    // the actor may sleep, and `onWake` restarts the loop.
    c.vars.lastPrice = await c.keepAwake(fetchPrice());
    await delay(5_000, c.abortSignal);
  }
}

const poller = actor({
  createVars: (): PollerVars => ({ lastPrice: 0, task: null }),

  // `onWake` runs on every start, so the loop is restarted after sleep
  onWake: (c) => {
    c.vars.task = pollPrices(c).catch((error) => {
      c.log.error({ msg: "price poller failed", error });
    });
  },

  // `c.abortSignal` fires on shutdown, so the loop exits on its own
  onSleep: async (c) => {
    await c.vars.task;
  },

  actions: {
    price: (c) => c.vars.lastPrice,
  },
});

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

async function fetchPrice(): Promise<number> {
  return 42;
}
