import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
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
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS drain_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          processed INTEGER NOT NULL,
          last_window_size INTEGER NOT NULL,
          last_window_total INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO drain_stats (id, processed, last_window_size, last_window_total) VALUES (1, 0, 0, 0)",
      );
    },
  }),
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
    getState: async (c) => {
      const rows = await c.db.execute<{
        processed: number;
        lastWindowSize: number;
        lastWindowTotal: number;
      }>(
        "SELECT processed, last_window_size AS lastWindowSize, last_window_total AS lastWindowTotal FROM drain_stats WHERE id = 1",
      );
      return rows[0];
    },
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

  await ctx.db.execute(
    "UPDATE drain_stats SET processed = processed + ?, last_window_size = ?, last_window_total = ? WHERE id = 1",
    window.length,
    window.length,
    windowTotal,
  );
}

export const registry = setup({ use: { boundedDrainActor } });
