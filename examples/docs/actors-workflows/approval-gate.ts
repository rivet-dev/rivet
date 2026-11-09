import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const approvalGateActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS order_status (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          status TEXT NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO order_status (id, status) VALUES (1, 'pending')",
      );
    },
  }),
  queues: {
    approval: queue<{ approved: boolean }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.step("validate-order", async (step) => {
      await validateOrder("order-123");
      await setStatus(step, "awaiting_approval");
    });

    const decision = await ctx.queue.next("wait-approval");

    if (decision.body.approved) {
      await ctx.step("fulfill-order", async (step) => {
        await fulfillOrder("order-123");
        await setStatus(step, "fulfilled");
      });
    } else {
      await ctx.step("cancel-order", async (step) => {
        await cancelOrder("order-123");
        await setStatus(step, "cancelled");
      });
    }
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ status: string }>(
        "SELECT status FROM order_status WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function setStatus(
  ctx: WorkflowStepContextOf<typeof approvalGateActor>,
  status: string,
): Promise<void> {
  await ctx.db.execute("UPDATE order_status SET status = ? WHERE id = 1", status);
}

async function validateOrder(orderId: string): Promise<void> {
  const res = await fetch(
    `https://api.example.com/orders/${orderId}/validate`,
    { method: "POST" },
  );
  if (!res.ok) throw new Error("Order validation failed");
}

async function fulfillOrder(orderId: string): Promise<void> {
  await fetch(`https://api.example.com/orders/${orderId}/fulfill`, {
    method: "POST",
  });
}

async function cancelOrder(orderId: string): Promise<void> {
  await fetch(`https://api.example.com/orders/${orderId}/cancel`, {
    method: "POST",
  });
}

export const registry = setup({ use: { approvalGateActor } });
