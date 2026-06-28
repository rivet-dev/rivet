import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const checkoutActor = actor({
  state: { status: "pending" as string },
  queues: {
    orders: queue<{ orderId: string }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("checkout-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-order");

        await loopCtx.rollbackCheckpoint("checkout-checkpoint");

        await loopCtx.step<string>({
          name: "reserve-inventory",
          run: (loopCtx) => reserveInventory(message.body.orderId),
          rollback: async (_rollbackCtx, id) => {
            await releaseInventory(id as string);
          },
        });

        await loopCtx.step<string>({
          name: "charge-card",
          run: (loopCtx) => chargeCard(message.body.orderId),
          rollback: async (_rollbackCtx, chargeId) => {
            await refundCharge(chargeId as string);
          },
        });

        await loopCtx.step("confirm", async (step) => {
          step.state.status = "confirmed";
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function reserveInventory(orderId: string): Promise<string> {
  const res = await fetch("https://api.example.com/inventory/reserve", {
    method: "POST",
    body: JSON.stringify({ orderId }),
  });
  return ((await res.json()) as { reservationId: string }).reservationId;
}

async function releaseInventory(reservationId: string): Promise<void> {
  await fetch(`https://api.example.com/inventory/${reservationId}/release`, {
    method: "POST",
  });
}

async function chargeCard(orderId: string): Promise<string> {
  const res = await fetch("https://api.stripe.com/v1/charges", {
    method: "POST",
    headers: { Authorization: `Bearer ${process.env.STRIPE_KEY}` },
    body: JSON.stringify({ orderId }),
  });
  return ((await res.json()) as { id: string }).id;
}

async function refundCharge(chargeId: string): Promise<void> {
  await fetch("https://api.stripe.com/v1/refunds", {
    method: "POST",
    headers: { Authorization: `Bearer ${process.env.STRIPE_KEY}` },
    body: JSON.stringify({ charge: chargeId }),
  });
}

export const registry = setup({ use: { checkoutActor } });
