import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const dashboardActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS dashboard_summary (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          users INTEGER NOT NULL,
          orders INTEGER NOT NULL,
          revenue INTEGER NOT NULL
        );
      `);
    },
  }),
  queues: {
    refresh: queue<Record<string, never>>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("dashboard-loop", async (loopCtx) => {
        await loopCtx.queue.next("wait-refresh");

        const summary = await loopCtx.join("fetch-summary", {
          users: {
            run: async (branchCtx) => {
              return await branchCtx.step("fetch-users", (branchCtx) => fetchCount("/users"));
            },
          },
          orders: {
            run: async (branchCtx) => {
              return await branchCtx.step("fetch-orders", (branchCtx) => fetchCount("/orders"));
            },
          },
          revenue: {
            run: async (branchCtx) => {
              return await branchCtx.step("fetch-revenue", (branchCtx) => fetchCount("/revenue"));
            },
          },
        });

        await loopCtx.step("save-summary", async (step) => {
          await step.db.execute(
            "INSERT OR REPLACE INTO dashboard_summary (id, users, orders, revenue) VALUES (1, ?, ?, ?)",
            summary.users,
            summary.orders,
            summary.revenue,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        users: number;
        orders: number;
        revenue: number;
      }>("SELECT users, orders, revenue FROM dashboard_summary WHERE id = 1");
      return { summary: rows[0] ?? null };
    },
  },
});

async function fetchCount(path: string): Promise<number> {
  const res = await fetch(`https://api.example.com${path}`);
  if (!res.ok) throw new Error(`fetch ${path} failed: ${res.status}`);
  return ((await res.json()) as { count: number }).count;
}

export const registry = setup({ use: { dashboardActor } });
