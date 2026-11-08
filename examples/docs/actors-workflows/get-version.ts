import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { workflow } from "rivetkit/workflow";

export const versionGateActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS process_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          processed INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO process_stats (id, processed) VALUES (1, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("process-loop", async (loopCtx) => {
        // Gate the changed code path. Each loop iteration resolves its version
        // independently: an iteration that already ran under the old code
        // (in-flight across the deploy) resolves to version 1, while a fresh
        // iteration resolves to `latest` (2 here). The resolved value is pinned
        // in history, so replays stay deterministic.
        const version = await loopCtx.getVersion("process-message", 2);

        if (version === 1) {
          // Preserve the original behavior for in-flight iterations.
          await loopCtx.step("process-v1", async (step) => {
            await step.db.execute(
              "UPDATE process_stats SET processed = processed + 1 WHERE id = 1",
            );
          });
        } else {
          // New behavior for iterations that begin after this deploy.
          await loopCtx.step("process-v2", async (step) => {
            await step.db.execute(
              "UPDATE process_stats SET processed = processed + 1 WHERE id = 1",
            );
          });
        }

        await loopCtx.sleep("idle", 1_000);
      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ processed: number }>(
        "SELECT processed FROM process_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
});

export const registry = setup({ use: { versionGateActor } });
