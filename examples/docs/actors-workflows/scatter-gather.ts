import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type ScatterMessage = { input: number };

export const shardActor = actor({
  actions: {
    compute: async (_c, input: number) => input * 10,
  },
});

export const scatterGatherActor = actor({
  state: {
    lastSum: 0,
  },
  run: workflow(async (ctx) => {
    await ctx.loop("scatter-gather-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-scatter", {
          timeout: 30_000,
        });

        if (!message) return;
        const scatter = message.body as ScatterMessage;

        const gathered = await loopCtx.join("gather", {
          shardA: {
            run: async (joinCtx) =>
              await joinCtx.step("call-shard-a", async (step) =>
                callShard(step, "a", scatter.input),
              ),
          },
          shardB: {
            run: async (joinCtx) =>
              await joinCtx.step("call-shard-b", async (step) =>
                callShard(step, "b", scatter.input),
              ),
          },
          shardC: {
            run: async (joinCtx) =>
              await joinCtx.step("call-shard-c", async (step) =>
                callShard(step, "c", scatter.input),
              ),
          },
        });

        await loopCtx.step("aggregate", async (step) => {
          step.state.lastSum = gathered.shardA + gathered.shardB + gathered.shardC;
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function callShard(
  ctx: WorkflowStepContextOf<typeof scatterGatherActor>,
  shardId: "a" | "b" | "c",
  input: number,
): Promise<number> {
  const client = ctx.client();
  const handle = client.shardActor.getOrCreate([shardId]);
  return await handle.compute(input);
}

export const registry = setup({ use: { scatterGatherActor, shardActor } });
