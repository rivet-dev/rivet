import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

async function chargeCard(orderId: string): Promise<string> {
  return `charge-${orderId}`;
}

export const timeoutActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS charge_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          last_charge_id TEXT
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO charge_stats (id, last_charge_id) VALUES (1, NULL)",
      );
    },
  }),
  queues: {
    charge: queue<{ orderId: string }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("charge-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-charge");

        const chargeId = await loopCtx.step<string>({
          name: "charge-card",
          timeout: 5_000,
          retryOnTimeout: true,
          maxRetries: 5,
          retryBackoffBase: 200,
          retryBackoffMax: 2_000,
          run: async (loopCtx) => await chargeCard(message.body.orderId),
        });

        await loopCtx.step("save-charge", async (step) => {
          await step.db.execute(
            "UPDATE charge_stats SET last_charge_id = ? WHERE id = 1",
            chargeId,
          );
        });

      });
  }),
});

export const registry = setup({ use: { timeoutActor } });
