import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

async function chargeCard(orderId: string): Promise<string> {
  return `charge-${orderId}`;
}

export const timeoutActor = actor({
  state: {
    lastChargeId: null as string | null,
  },
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
          step.state.lastChargeId = chargeId;
        });

      });
  }),
});

export const registry = setup({ use: { timeoutActor } });
