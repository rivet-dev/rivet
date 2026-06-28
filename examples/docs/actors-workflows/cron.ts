import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

function nextMinute(timestamp: number): number {
  const minuteMs = 60_000;
  return Math.floor(timestamp / minuteMs) * minuteMs + minuteMs;
}

export const cronActor = actor({
  state: {
    runs: 0,
    lastRunAt: null as number | null,
  },
  queues: {
    "cron-tick": queue<{ scheduledAt: number }>(),
  },
  onCreate: async (c) => {
    const firstTickAt = nextMinute(Date.now());
    await c.schedule.at(firstTickAt, "enqueueCronTick", firstTickAt);
  },
  actions: {
    enqueueCronTick: async (c, scheduledAt: number) => {
      await c.queue.send("cron-tick", { scheduledAt });

      const nextTickAt = nextMinute(scheduledAt + 1);
      await c.schedule.at(nextTickAt, "enqueueCronTick", nextTickAt);
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
