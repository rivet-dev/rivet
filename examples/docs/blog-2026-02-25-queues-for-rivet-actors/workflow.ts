import { actor, queue, setup } from "rivetkit";
import { workflow } from "rivetkit/workflow";

const worker = actor({
  queues: {
    orders: queue<{ orderId: string }>(),
  },
  // Workflow replays safely on crash or restart
  run: workflow(async (ctx) => {
    await ctx.loop("order-loop", async (loopCtx) => {
      const message = await loopCtx.queue.next("wait-order");
      await loopCtx.step("charge", async () =>
        charge(message.body.orderId),
      );
      await loopCtx.step("fulfill", async () =>
        fulfill(message.body.orderId),
      );
      await loopCtx.step("notify", async () =>
        notify(message.body.orderId),
      );
    });
  }),
});

async function charge(orderId: string) { /* ... */ }
async function fulfill(orderId: string) { /* ... */ }
async function notify(orderId: string) { /* ... */ }

export const registry = setup({ use: { worker } });
