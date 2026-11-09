import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type PaymentMessage = { id: string; amount: number };

export const checkpointFriendlyActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS payment_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          applied_count INTEGER NOT NULL,
          total_amount INTEGER NOT NULL,
          last_payment_id TEXT
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO payment_stats (id, applied_count, total_amount, last_payment_id) VALUES (1, 0, 0, NULL)",
      );
    },
  }),
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
          await step.db.execute(
            "UPDATE payment_stats SET applied_count = applied_count + 1, total_amount = total_amount + ?, last_payment_id = ? WHERE id = 1",
            plan.amount,
            plan.paymentId,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        appliedCount: number;
        totalAmount: number;
        lastPaymentId: string | null;
      }>(
        "SELECT applied_count AS appliedCount, total_amount AS totalAmount, last_payment_id AS lastPaymentId FROM payment_stats WHERE id = 1",
      );
      return rows[0];
    },
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
