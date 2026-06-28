import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const dashboardActor = actor({
  state: {
    summary: null as null | {
      users: number;
      orders: number;
      revenue: number;
    },
  },
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
          step.state.summary = summary;
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function fetchCount(path: string): Promise<number> {
  const res = await fetch(`https://api.example.com${path}`);
  if (!res.ok) throw new Error(`fetch ${path} failed: ${res.status}`);
  return ((await res.json()) as { count: number }).count;
}

export const registry = setup({ use: { dashboardActor } });
