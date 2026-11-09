import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const fanInOutActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS fan_in_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          total INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO fan_in_stats (id, total) VALUES (1, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("join-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-refresh", {
          timeout: 30_000,
        });

        if (!message) return;

        const joined = await loopCtx.join("parallel-work", {
          users: {
            run: async (branchCtx) =>
              await branchCtx.step("fetch-users", (branchCtx) => fetchCount("/users")),
          },
          orders: {
            run: async (branchCtx) =>
              await branchCtx.step("fetch-orders", (branchCtx) => fetchCount("/orders")),
          },
          invoices: {
            run: async (branchCtx) =>
              await branchCtx.step("fetch-invoices", (branchCtx) => fetchCount("/invoices")),
          },
        });

        await loopCtx.step("merge-results", async (step) => {
          await step.db.execute(
            "UPDATE fan_in_stats SET total = ? WHERE id = 1",
            joined.users + joined.orders + joined.invoices,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ total: number }>(
        "SELECT total FROM fan_in_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function fetchCount(path: string): Promise<number> {
  const res = await fetch(`https://api.example.com${path}`);
  if (!res.ok) throw new Error(`fetch ${path} failed: ${res.status}`);
  return ((await res.json()) as { count: number }).count;
}

export const registry = setup({ use: { fanInOutActor } });
