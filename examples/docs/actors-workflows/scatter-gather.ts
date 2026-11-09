import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type ScatterMessage = { input: number };

export const shardActor = actor({
  actions: {
    compute: async (_c, input: number) => input * 10,
  },
});

export const scatterGatherActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS gather_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          last_sum INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO gather_stats (id, last_sum) VALUES (1, 0)",
      );
    },
  }),
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
          await step.db.execute(
            "UPDATE gather_stats SET last_sum = ? WHERE id = 1",
            gathered.shardA + gathered.shardB + gathered.shardC,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ lastSum: number }>(
        "SELECT last_sum AS lastSum FROM gather_stats WHERE id = 1",
      );
      return rows[0];
    },
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
