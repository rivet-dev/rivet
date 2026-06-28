import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const fanInOutActor = actor({
  state: {
    total: 0,
  },
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
          step.state.total =
            joined.users + joined.orders + joined.invoices;
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

export const registry = setup({ use: { fanInOutActor } });
