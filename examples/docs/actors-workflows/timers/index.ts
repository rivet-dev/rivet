import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type Reminder = {
  text: string;
  at: number;
};

export const reminderActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS fired_reminders (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          text TEXT NOT NULL
        );
      `);
    },
  }),
  queues: {
    reminders: queue<Reminder>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("reminder-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-reminder");

        const runAt = Math.max(Date.now(), message.body.at);
        await loopCtx.sleepUntil("wait-until-reminder", runAt);

        await loopCtx.step("record-reminder", async (step) => {
          await step.db.execute(
            "INSERT INTO fired_reminders (text) VALUES (?)",
            message.body.text,
          );
        });

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ text: string }>(
        "SELECT text FROM fired_reminders ORDER BY id",
      );
      return { fired: rows.map((row) => row.text) };
    },
  },
});

export const registry = setup({ use: { reminderActor } });
