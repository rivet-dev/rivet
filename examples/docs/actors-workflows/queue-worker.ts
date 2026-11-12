import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type Job = { id: string; amount: number };

export const queueWorkerActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS job_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          processed INTEGER NOT NULL,
          total_amount INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO job_stats (id, processed, total_amount) VALUES (1, 0, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("worker-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-job", {
          timeout: 30_000,
        });

        if (!message) return;
        const job = message.body as Job;

        await loopCtx.step("process-job", async (step) => {
          await step.db.execute(
            "UPDATE job_stats SET processed = processed + 1, total_amount = total_amount + ? WHERE id = 1",
            job.amount,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ processed: number; totalAmount: number }>(
        "SELECT processed, total_amount AS totalAmount FROM job_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
});

export const registry = setup({ use: { queueWorkerActor } });
