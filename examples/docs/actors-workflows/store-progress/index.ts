import { actor, event, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type Progress = {
  stage: "idle" | "running" | "completed";
  completed: number;
  total: number;
};

export const progressActor = actor({
  state: {
    progress: {
      stage: "idle",
      completed: 0,
      total: 0,
    } as Progress,
    sum: 0,
  },
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
    getState: (c) => c.state,
  },
});

function markProgressRunning(ctx: WorkflowStepContextOf<typeof progressActor>): void {
  ctx.state.progress = {
    stage: "running",
    completed: ctx.state.progress.completed,
    total: ctx.state.progress.total + 1,
  };
  ctx.broadcast("progressUpdated", ctx.state.progress);
}

function applyProgressJob(
  ctx: WorkflowStepContextOf<typeof progressActor>,
  value: number,
): void {
  ctx.state.sum += value;
  ctx.state.progress = {
    stage: "completed",
    completed: ctx.state.progress.completed + 1,
    total: ctx.state.progress.total,
  };
  ctx.broadcast("progressUpdated", ctx.state.progress);
}

export const registry = setup({ use: { progressActor } });
