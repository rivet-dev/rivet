import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const versionedWorkflowActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS run_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          runs INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO run_stats (id, runs) VALUES (1, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.step("validate-v2", async (step) => {
      await step.db.execute("UPDATE run_stats SET runs = runs + 1 WHERE id = 1");
    });

    await ctx.removed("validate-v1", "step");

    await ctx.loop("main-loop-v2", async (loopCtx) => {
        await loopCtx.sleep("idle", 500);
        await loopCtx.step("heartbeat-v2", async (step) => {
          await step.db.execute("UPDATE run_stats SET runs = runs + 1 WHERE id = 1");
        });
      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ runs: number }>(
        "SELECT runs FROM run_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
});

export const registry = setup({ use: { versionedWorkflowActor } });
