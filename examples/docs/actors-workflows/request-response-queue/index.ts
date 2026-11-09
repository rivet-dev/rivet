import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type RequestMessage = { value: number };

export const requestResponseActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS request_stats (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          handled INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO request_stats (id, handled) VALUES (1, 0)",
      );
    },
  }),
  queues: {
    requests: queue<RequestMessage, { doubled: number }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("request-response-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-request", {
          completable: true,
        });

        if (!message.complete) return;

        const doubled = await loopCtx.step("handle-request", async (step) => {
          await step.db.execute(
            "UPDATE request_stats SET handled = handled + 1 WHERE id = 1",
          );
          return message.body.value * 2;
        });

        await message.complete({ doubled });
      });
  }),
});

export const registry = setup({ use: { requestResponseActor } });
