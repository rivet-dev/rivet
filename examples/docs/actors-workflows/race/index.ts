import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const auctionActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS auction_result (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          result TEXT
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO auction_result (id, result) VALUES (1, NULL)",
      );
    },
  }),
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
      await step.db.execute(
        "UPDATE auction_result SET result = ? WHERE id = 1",
        winner === "bid" ? "sold" : "expired",
      );
    });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ result: "sold" | "expired" | null }>(
        "SELECT result FROM auction_result WHERE id = 1",
      );
      return rows[0];
    },
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
