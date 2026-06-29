import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type WorkMessage = { id: string; value: number };

const MAX_PER_ITERATION = 10;
const CONCURRENCY_LIMIT = 3;

async function processWork(value: number): Promise<number> {
  return value * 2;
}

async function runWithLimit<T>(
  limit: number,
  items: T[],
  fn: (item: T) => Promise<void>,
): Promise<void> {
  let nextIndex = 0;
  const workers = Array.from({ length: limit }, async () => {
    while (nextIndex < items.length) {
      const current = items[nextIndex];
      nextIndex += 1;
      await fn(current);
    }
  });
  await Promise.all(workers);
}

export const boundedDrainActor = actor({
  state: {
    processed: 0,
    lastWindowSize: 0,
    lastWindowTotal: 0,
  },
  run: workflow(async (ctx) => {
    await ctx.loop("bounded-drain-loop", async (loopCtx) => {
        const window: WorkMessage[] = [];

        for (let i = 0; i < MAX_PER_ITERATION; i += 1) {
          const [message] = await loopCtx.queue.nextBatch("wait-work", {
            timeout: i === 0 ? 30_000 : 10,
          });
          if (!message) break;
          window.push(message.body as WorkMessage);
        }

        if (window.length === 0) return;

        await loopCtx.step("process-window", async (step) =>
          processWindow(step, window),
        );

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function processWindow(
  ctx: WorkflowStepContextOf<typeof boundedDrainActor>,
  window: WorkMessage[],
): Promise<void> {
  let windowTotal = 0;
  await runWithLimit(CONCURRENCY_LIMIT, window, async (work) => {
    const result = await processWork(work.value);
    windowTotal += result;
  });

  ctx.state.processed += window.length;
  ctx.state.lastWindowSize = window.length;
  ctx.state.lastWindowTotal = windowTotal;
}

export const registry = setup({ use: { boundedDrainActor } });
