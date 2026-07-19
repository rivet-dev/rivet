import { actor, queue, setup, type ScheduledFireInfo } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const cronActor = actor({
  state: {
    runs: 0,
    lastRunAt: null as number | null,
  },
  queues: {
    "cron-tick": queue<{ scheduledAt: number }>(),
  },
  onCreate: async (c) => {
    await c.cron.every({
      name: "workflow-tick",
      intervalMs: 60_000,
      action: "enqueueCronTick",
      args: [],
      maxHistory: 100,
    });
  },
  actions: {
    enqueueCronTick: async (c, fire: ScheduledFireInfo) => {
      await c.queue.send("cron-tick", { scheduledAt: fire.scheduledAt });
    },
    getState: (c) => c.state,
  },
  run: workflow(async (ctx) => {
    await ctx.loop("cron-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-cron-tick");

        await loopCtx.step("run-cron-job", async (step) => {
          step.state.runs += 1;
          step.state.lastRunAt = message.body.scheduledAt;
        });

      });
  }),
});

export const registry = setup({ use: { cronActor } });
