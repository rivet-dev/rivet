import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type CheckoutMessage = {
  orderId: string;
  amount: number;
};

export const inventoryActor = actor({
  actions: {
    reserve: async (_c, orderId: string) => `reserve-${orderId}`,
    release: async (_c, reservationId: string) => reservationId,
  },
});

export const billingActor = actor({
  actions: {
    charge: async (_c, amount: number) => `charge-${amount}`,
    refund: async (_c, chargeId: string) => chargeId,
  },
});

export const checkoutSagaActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS saga_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          completed_orders INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO saga_stats (id, completed_orders) VALUES (1, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("checkout-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-order", {
          timeout: 30_000,
        });

        if (!message) return;
        const checkout = message.body as CheckoutMessage;

        await loopCtx.rollbackCheckpoint("checkout-saga");

        await loopCtx.step({
          name: "reserve-inventory",
          run: async (ctx) => reserveInventoryForCheckout(ctx, checkout.orderId),
          // Rollback callbacks only receive a rollback context, not actor
          // APIs like client(). Compensate with direct external calls.
          rollback: async (_rollbackCtx, output) => {
            await releaseInventoryForCheckout(output as string);
          },
        });

        await loopCtx.step({
          name: "charge-card",
          run: async (ctx) => chargeCheckout(ctx, checkout.amount),
          rollback: async (_rollbackCtx, output) => {
            await refundCheckout(output as string);
          },
        });

        await loopCtx.step("mark-complete", async (step) =>
          markOrderComplete(step),
        );

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ completedOrders: number }>(
        "SELECT completed_orders AS completedOrders FROM saga_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function reserveInventoryForCheckout(
  ctx: WorkflowStepContextOf<typeof checkoutSagaActor>,
  orderId: string,
): Promise<string> {
  const client = ctx.client();
  const inventory = client.inventoryActor.getOrCreate(["main"]);
  return await inventory.reserve(orderId);
}

async function releaseInventoryForCheckout(
  reservationId: string,
): Promise<void> {
  await fetch("https://api.example.com/inventory/release", {
    method: "POST",
    body: JSON.stringify({ reservationId }),
  });
}

async function chargeCheckout(
  ctx: WorkflowStepContextOf<typeof checkoutSagaActor>,
  amount: number,
): Promise<string> {
  const client = ctx.client();
  const billing = client.billingActor.getOrCreate(["main"]);
  return await billing.charge(amount);
}

async function refundCheckout(
  chargeId: string,
): Promise<void> {
  await fetch("https://api.example.com/billing/refund", {
    method: "POST",
    body: JSON.stringify({ chargeId }),
  });
}

async function markOrderComplete(
  ctx: WorkflowStepContextOf<typeof checkoutSagaActor>,
): Promise<void> {
  await ctx.db.execute(
    "UPDATE saga_stats SET completed_orders = completed_orders + 1 WHERE id = 1",
  );
}

export const registry = setup({
  use: { checkoutSagaActor, inventoryActor, billingActor },
});
