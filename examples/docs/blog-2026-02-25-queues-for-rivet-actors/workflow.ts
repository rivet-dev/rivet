import { actor, queue, setup } from "rivetkit";
import { workflow } from "rivetkit/workflow";

const worker = actor({
  state: { processed: 0 },
  queues: {
    orders: queue<{ orderId: string }>(),
  },
  // Workflow replays safely on crash or restart
  run: workflow(async (ctx) => {
    for await (const message of ctx.queue.iter()) {
      await ctx.step("charge", async (ctx) =>
        charge(message.body.orderId),
      );
      await ctx.step("fulfill", async (ctx) =>
        fulfill(message.body.orderId),
      );
      await ctx.step("notify", async (ctx) =>
        notify(message.body.orderId),
      );
    }
  }),
});

async function charge(orderId: string) { /* ... */ }
async function fulfill(orderId: string) { /* ... */ }
async function notify(orderId: string) { /* ... */ }

export const registry = setup({ use: { worker } });
