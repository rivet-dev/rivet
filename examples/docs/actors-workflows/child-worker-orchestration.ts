import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type BatchMessage = { payload: number };

export const childWorkerActor = actor({
  actions: {
    process: async (_c, payload: number) => payload * 3,
  },
});

export const orchestratorActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS orchestrator_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          last_total INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO orchestrator_stats (id, last_total) VALUES (1, 0)",
      );
    },
  }),
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
          await step.db.execute(
            "UPDATE orchestrator_stats SET last_total = ? WHERE id = 1",
            results.a + results.b + results.c,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ lastTotal: number }>(
        "SELECT last_total AS lastTotal FROM orchestrator_stats WHERE id = 1",
      );
      return rows[0];
    },
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
