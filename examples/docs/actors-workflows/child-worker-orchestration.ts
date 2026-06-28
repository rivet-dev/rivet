import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type BatchMessage = { payload: number };

export const childWorkerActor = actor({
  actions: {
    process: async (_c, payload: number) => payload * 3,
  },
});

export const orchestratorActor = actor({
  state: {
    lastTotal: 0,
  },
  run: workflow(async (ctx) => {
    await ctx.step("start-children", async (step) => startChildren(step));

    await ctx.loop("orchestrate-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-batch", {
          timeout: 30_000,
        });

        if (!message) return;
        const batch = message.body as BatchMessage;

        const results = await loopCtx.join("collect-updates", {
          a: {
            run: async (joinCtx) =>
              await joinCtx.step("run-child-a", async (step) =>
                runChildWorker(step, "child-a", batch.payload),
              ),
          },
          b: {
            run: async (joinCtx) =>
              await joinCtx.step("run-child-b", async (step) =>
                runChildWorker(step, "child-b", batch.payload),
              ),
          },
          c: {
            run: async (joinCtx) =>
              await joinCtx.step("run-child-c", async (step) =>
                runChildWorker(step, "child-c", batch.payload),
              ),
          },
        });

        await loopCtx.step("reconcile", async (step) => {
          step.state.lastTotal = results.a + results.b + results.c;
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function startChildren(
  ctx: WorkflowStepContextOf<typeof orchestratorActor>,
): Promise<void> {
  const client = ctx.client();
  await client.childWorkerActor.getOrCreate(["child-a"]).process(0);
  await client.childWorkerActor.getOrCreate(["child-b"]).process(0);
  await client.childWorkerActor.getOrCreate(["child-c"]).process(0);
}

async function runChildWorker(
  ctx: WorkflowStepContextOf<typeof orchestratorActor>,
  workerId: "child-a" | "child-b" | "child-c",
  payload: number,
): Promise<number> {
  const client = ctx.client();
  return await client.childWorkerActor.getOrCreate([workerId]).process(payload);
}

export const registry = setup({ use: { orchestratorActor, childWorkerActor } });
