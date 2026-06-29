import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const approvalGateActor = actor({
  state: { status: "pending" as string },
  queues: {
    approval: queue<{ approved: boolean }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.step("validate-order", async (step) => {
      await validateOrder("order-123");
      step.state.status = "awaiting_approval";
    });

    const decision = await ctx.queue.next("wait-approval");

    if (decision.body.approved) {
      await ctx.step("fulfill-order", async (step) => {
        await fulfillOrder("order-123");
        step.state.status = "fulfilled";
      });
    } else {
      await ctx.step("cancel-order", async (step) => {
        await cancelOrder("order-123");
        step.state.status = "cancelled";
      });
    }
  }),
  actions: {
    getState: (c) => c.state,
  },
});

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
