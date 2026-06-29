import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type PaymentMessage = { id: string; amount: number };

export const checkpointFriendlyActor = actor({
  state: {
    appliedCount: 0,
    totalAmount: 0,
    lastPaymentId: null as string | null,
  },
  run: workflow(async (ctx) => {
    await ctx.loop("payment-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-payment", {
          timeout: 30_000,
        });

        if (!message) return;
        const payment = message.body as PaymentMessage;

        await loopCtx.rollbackCheckpoint("apply-payment-checkpoint");

        const plan = (await loopCtx.step("build-plan", async (loopCtx) =>
          buildPaymentPlan(payment),
        )) as { paymentId: string; amount: number };

        await loopCtx.step("apply-side-effects", async (step) => {
          step.state.appliedCount += 1;
          step.state.totalAmount += plan.amount;
          step.state.lastPaymentId = plan.paymentId;
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

function buildPaymentPlan(payment: PaymentMessage): {
  paymentId: string;
  amount: number;
} {
  return {
    paymentId: payment.id,
    amount: payment.amount,
  };
}

export const registry = setup({ use: { checkpointFriendlyActor } });
