import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { workflow } from "rivetkit/workflow";

export const paymentActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS payment_status (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          status TEXT NOT NULL,
          reason TEXT
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO payment_status (id, status, reason) VALUES (1, 'pending', NULL)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    const charge = await ctx.tryStep({
      name: "charge-card",
      maxRetries: 3,
      run: async (ctx) => await chargeCard("order-123"),
    });

    await ctx.step("store-charge-result", async (step) => {
      if (!charge.ok) {
        await step.db.execute(
          "UPDATE payment_status SET status = 'manual-review', reason = ? WHERE id = 1",
          charge.failure.error.message,
        );
        return;
      }

      await step.db.execute(
        "UPDATE payment_status SET status = 'paid', reason = NULL WHERE id = 1",
      );
    });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        status: "pending" | "manual-review" | "paid";
        reason: string | null;
      }>(
        "SELECT status, reason FROM payment_status WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function chargeCard(orderId: string): Promise<string> {
  return `charge-${orderId}`;
}

export const registry = setup({ use: { paymentActor } });
