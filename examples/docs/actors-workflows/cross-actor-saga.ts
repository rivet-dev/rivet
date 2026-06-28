import { actor, setup } from "rivetkit";
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
  state: {
    completedOrders: 0,
  },
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
    getState: (c) => c.state,
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

function markOrderComplete(
  ctx: WorkflowStepContextOf<typeof checkoutSagaActor>,
): void {
  ctx.state.completedOrders += 1;
}

export const registry = setup({
  use: { checkoutSagaActor, inventoryActor, billingActor },
});
