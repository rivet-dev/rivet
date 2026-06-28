import { actor, queue, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type Reminder = {
  text: string;
  at: number;
};

export const reminderActor = actor({
  state: {
    fired: [] as string[],
  },
  queues: {
    reminders: queue<Reminder>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("reminder-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-reminder");

        const runAt = Math.max(Date.now(), message.body.at);
        await loopCtx.sleepUntil("wait-until-reminder", runAt);

        await loopCtx.step("record-reminder", async (step) => {
          step.state.fired.push(message.body.text);
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

export const registry = setup({ use: { reminderActor } });
