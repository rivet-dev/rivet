import { actor, event, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type Progress = {
  stage: "idle" | "running" | "completed";
  completed: number;
  total: number;
};

export const progressActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS job_progress (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          stage TEXT NOT NULL,
          completed INTEGER NOT NULL,
          total INTEGER NOT NULL,
          sum INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO job_progress (id, stage, completed, total, sum) VALUES (1, 'idle', 0, 0, 0)",
      );
    },
  }),
  events: {
    progressUpdated: event<Progress>(),
  },
  queues: {
    jobs: queue<{ value: number }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("progress-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-job");

        await loopCtx.step("mark-running", async (step) =>
          markProgressRunning(step),
        );

        await loopCtx.step("apply-job", async (step) =>
          applyProgressJob(step, message.body.value),
        );

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<Progress & { sum: number }>(
        "SELECT stage, completed, total, sum FROM job_progress WHERE id = 1",
      );
      const { sum, ...progress } = rows[0];
      return { progress, sum };
    },
  },
});

async function markProgressRunning(
  ctx: WorkflowStepContextOf<typeof progressActor>,
): Promise<void> {
  const rows = await ctx.db.execute<Progress>(
    "UPDATE job_progress SET stage = 'running', total = total + 1 WHERE id = 1 RETURNING stage, completed, total",
  );
  ctx.broadcast("progressUpdated", rows[0]);
}

async function applyProgressJob(
  ctx: WorkflowStepContextOf<typeof progressActor>,
  value: number,
): Promise<void> {
  const rows = await ctx.db.execute<Progress>(
    "UPDATE job_progress SET sum = sum + ?, stage = 'completed', completed = completed + 1 WHERE id = 1 RETURNING stage, completed, total",
    value,
  );
  ctx.broadcast("progressUpdated", rows[0]);
}

export const registry = setup({ use: { progressActor } });
