import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type MetricMessage = { value: number };

export const batchDrainerActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS pending_metrics (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          value INTEGER NOT NULL
        );
      `);
      await db.execute(`
        CREATE TABLE IF NOT EXISTS batch_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          flushed_batches INTEGER NOT NULL,
          last_batch_total INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO batch_stats (id, flushed_batches, last_batch_total) VALUES (1, 0, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("drain-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-metric", {
          timeout: 5_000,
        });

        const pendingCount = await loopCtx.step(
          "buffer-message",
          async (step) => {
            if (message) {
              await step.db.execute(
                "INSERT INTO pending_metrics (value) VALUES (?)",
                (message.body as MetricMessage).value,
              );
            }
            const rows = await step.db.execute<{ count: number }>(
              "SELECT COUNT(*) AS count FROM pending_metrics",
            );
            return rows[0].count;
          },
        );

        if (pendingCount < 5) return;

        await loopCtx.step("flush-batch", async (step) => flushBatch(step));

      });
  }),
  actions: {
    getState: async (c) => {
      const pending = await c.db.execute<{ value: number }>(
        "SELECT value FROM pending_metrics ORDER BY id",
      );
      const stats = await c.db.execute<{ flushedBatches: number; lastBatchTotal: number }>(
        "SELECT flushed_batches AS flushedBatches, last_batch_total AS lastBatchTotal FROM batch_stats WHERE id = 1",
      );
      return { pending: pending.map((row) => row.value), ...stats[0] };
    },
  },
});

async function flushBatch(
  ctx: WorkflowStepContextOf<typeof batchDrainerActor>,
): Promise<void> {
  await ctx.db.transaction(
    async (tx) => {
      const rows = await tx.execute<{ total: number }>(
        "SELECT COALESCE(SUM(value), 0) AS total FROM pending_metrics",
      );
      await tx.execute(
        "UPDATE batch_stats SET last_batch_total = ?, flushed_batches = flushed_batches + 1 WHERE id = 1",
        rows[0].total,
      );
      await tx.execute("DELETE FROM pending_metrics");
    },
    { name: "flush-batch" },
  );
}

export const registry = setup({ use: { batchDrainerActor } });
