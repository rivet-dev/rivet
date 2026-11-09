import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

async function pollExternal(attempt: number): Promise<boolean> {
  return attempt % 3 === 0;
}

export const pollBackoffActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS poll_status (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          attempts INTEGER NOT NULL,
          backoff_ms INTEGER NOT NULL,
          status TEXT NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO poll_status (id, attempts, backoff_ms, status) VALUES (1, 0, 100, 'unknown')",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("poll-loop", async (loopCtx) => {
        const success = await loopCtx.step("poll-target", async (step) => {
          const rows = await step.db.execute<{ attempts: number }>(
            "UPDATE poll_status SET attempts = attempts + 1 WHERE id = 1 RETURNING attempts",
          );
          return pollExternal(rows[0].attempts);
        });

        if (success) {
          await loopCtx.step("reset-backoff", async (step) => {
            await step.db.execute(
              "UPDATE poll_status SET status = 'healthy', backoff_ms = 100 WHERE id = 1",
            );
          });
          await loopCtx.sleep("healthy-interval", 1_000);
          return;
        }

        const retryDelay = await loopCtx.step("grow-backoff", async (step) => {
          const rows = await step.db.execute<{ backoffMs: number }>(
            "UPDATE poll_status SET status = 'retrying', backoff_ms = MIN(backoff_ms * 2, 5000) WHERE id = 1 RETURNING backoff_ms AS backoffMs",
          );
          return rows[0].backoffMs;
        });

        await loopCtx.sleep("retry-delay", retryDelay);
      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        attempts: number;
        backoffMs: number;
        status: "unknown" | "healthy" | "retrying";
      }>(
        "SELECT attempts, backoff_ms AS backoffMs, status FROM poll_status WHERE id = 1",
      );
      return rows[0];
    },
  },
});

export const registry = setup({ use: { pollBackoffActor } });
