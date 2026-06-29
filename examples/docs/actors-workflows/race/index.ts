import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const auctionActor = actor({
  state: { result: null as "sold" | "expired" | null },
  queues: {
    bids: queue<{ amount: number }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.step("list-item", (ctx) => listItem("item-123"));

    const { winner } = await ctx.race("bid-or-expire", [
      {
        name: "bid",
        run: async (branchCtx) => {
          const bid = await branchCtx.queue.next("wait-bid");
          return bid.body.amount;
        },
      },
      {
        name: "expire",
        run: async (branchCtx) => {
          await branchCtx.sleep("auction-timeout", 24 * 60 * 60 * 1000);
          return 0;
        },
      },
    ]);

    await ctx.step("finalize", async (step) => {
      await finalizeAuction("item-123", winner);
      step.state.result = winner === "bid" ? "sold" : "expired";
    });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function listItem(itemId: string): Promise<void> {
  await fetch(`https://api.example.com/auctions/${itemId}`, {
    method: "POST",
  });
}

async function finalizeAuction(
  itemId: string,
  outcome: string,
): Promise<void> {
  await fetch(`https://api.example.com/auctions/${itemId}/finalize`, {
    method: "POST",
    body: JSON.stringify({ outcome }),
  });
}

export const registry = setup({ use: { auctionActor } });
