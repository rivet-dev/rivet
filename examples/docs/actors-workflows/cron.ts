import { actor, queue, setup, type ScheduledFireInfo } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const cronActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS cron_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          runs INTEGER NOT NULL,
          last_run_at INTEGER
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO cron_stats (id, runs, last_run_at) VALUES (1, 0, NULL)",
      );
    },
  }),
  queues: {
    "cron-tick": queue<{ scheduledAt: number }>(),
  },
  onCreate: async (c) => {
    await c.cron.every({
      name: "workflow-tick",
      interval: 60_000,
      action: "enqueueCronTick",
      args: [],
      maxHistory: 100,
    });
  },
  actions: {
    enqueueCronTick: async (c, fire: ScheduledFireInfo) => {
      await c.queue.send("cron-tick", { scheduledAt: fire.scheduledAt });
    },
    getState: async (c) => {
      const rows = await c.db.execute<{ runs: number; lastRunAt: number | null }>(
        "SELECT runs, last_run_at AS lastRunAt FROM cron_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
  run: workflow(async (ctx) => {
    await ctx.loop("cron-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-cron-tick");

        await loopCtx.step("run-cron-job", async (step) => {
          await step.db.execute(
            "UPDATE cron_stats SET runs = runs + 1, last_run_at = ? WHERE id = 1",
            message.body.scheduledAt,
          );
        });

      });
  }),
});

export const registry = setup({ use: { cronActor } });
