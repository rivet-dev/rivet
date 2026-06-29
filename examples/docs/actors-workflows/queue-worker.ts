import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type Job = { id: string; amount: number };

export const queueWorkerActor = actor({
  state: {
    processed: 0,
    totalAmount: 0,
  },
  run: workflow(async (ctx) => {
    await ctx.loop("worker-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-job", {
          timeout: 30_000,
        });

        if (!message) return;
        const job = message.body as Job;

        await loopCtx.step("process-job", async (step) => {
          step.state.processed += 1;
          step.state.totalAmount += job.amount;
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

export const registry = setup({ use: { queueWorkerActor } });
