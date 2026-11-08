import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type TaskMessage = {
  taskId: string;
  workerId: string;
  value: number;
};

export const workerActor = actor({
  actions: {
    runTask: async (_c, value: number) => value * 2,
  },
});

export const coordinatorActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS coordinator_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          last_task_id TEXT,
          last_result INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO coordinator_stats (id, last_task_id, last_result) VALUES (1, NULL, 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("orchestrator-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-task", {
          timeout: 30_000,
        });

        if (!message) return;
        const task = message.body as TaskMessage;

        const result = await loopCtx.step("dispatch-rpc", async (step) =>
          dispatchTask(step, task),
        );

        await loopCtx.step("record-result", async (step) => {
          await step.db.execute(
            "UPDATE coordinator_stats SET last_task_id = ?, last_result = ? WHERE id = 1",
            task.taskId,
            result as number,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ lastTaskId: string | null; lastResult: number }>(
        "SELECT last_task_id AS lastTaskId, last_result AS lastResult FROM coordinator_stats WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function dispatchTask(
  ctx: WorkflowStepContextOf<typeof coordinatorActor>,
  task: TaskMessage,
): Promise<number> {
  const client = ctx.client();
  const worker = client.workerActor.getOrCreate([task.workerId]);
  return await worker.runTask(task.value);
}

export const registry = setup({ use: { coordinatorActor, workerActor } });
