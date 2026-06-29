import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type MetricMessage = { value: number };

export const batchDrainerActor = actor({
  state: {
    pending: [] as number[],
    flushedBatches: 0,
    lastBatchTotal: 0,
  },
  run: workflow(async (ctx) => {
    await ctx.loop("drain-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-metric", {
          timeout: 5_000,
        });

        const pendingCount = await loopCtx.step(
          "buffer-message",
          async (step) => {
            if (message) {
              step.state.pending.push((message.body as MetricMessage).value);
            }
            return step.state.pending.length;
          },
        );

        if (pendingCount < 5) return;

        await loopCtx.step("flush-batch", async (step) => flushBatch(step));

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

function flushBatch(ctx: WorkflowStepContextOf<typeof batchDrainerActor>): void {
  const total = ctx.state.pending.reduce(
    (sum: number, value: number) => sum + value,
    0,
  );
  ctx.state.lastBatchTotal = total;
  ctx.state.flushedBatches += 1;
  ctx.state.pending = [];
}

export const registry = setup({ use: { batchDrainerActor } });
