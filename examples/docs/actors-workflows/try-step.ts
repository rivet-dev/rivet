import { actor, setup } from "rivetkit";
import { workflow } from "rivetkit/workflow";

export const paymentActor = actor({
  state: {
    status: "pending" as "pending" | "manual-review" | "paid",
    reason: null as string | null,
  },
  run: workflow(async (ctx) => {
    const charge = await ctx.tryStep({
      name: "charge-card",
      maxRetries: 3,
      run: async (ctx) => await chargeCard("order-123"),
    });

    await ctx.step("store-charge-result", async (step) => {
      if (!charge.ok) {
        step.state.status = "manual-review";
        step.state.reason = charge.failure.error.message;
        return;
      }

      step.state.status = "paid";
      step.state.reason = null;
    });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function chargeCard(orderId: string): Promise<string> {
  return `charge-${orderId}`;
}

export const registry = setup({ use: { paymentActor } });
