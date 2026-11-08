import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const workflowCounter = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS counter (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          value INTEGER NOT NULL,
          processed INTEGER NOT NULL,
          last_operation_id TEXT
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO counter (id, value, processed, last_operation_id) VALUES (1, 0, 0, NULL)",
      );
    },
  }),
  queues: {
    counter: queue<{ delta: number }>(),
  },
  run: workflow(async (ctx) => {
    await ctx.loop("counter-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-counter-command");

        await loopCtx.step("apply-counter-command", async (step) =>
          applyCounterCommand(step, message.body.delta),
        );

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        value: number;
        processed: number;
        lastOperationId: string | null;
      }>(
        "SELECT value, processed, last_operation_id AS lastOperationId FROM counter WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function applyCounterCommand(
  ctx: WorkflowStepContextOf<typeof workflowCounter>,
  delta: number,
): Promise<void> {
  const response = await fetch("https://api.example.com/counter/apply", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ delta }),
  });
  if (!response.ok) {
    throw new Error(`counter apply failed: ${response.status}`);
  }
  const result = (await response.json()) as {
    nextValue: number;
    operationId: string;
  };
  await ctx.db.execute(
    "UPDATE counter SET value = ?, last_operation_id = ?, processed = processed + 1 WHERE id = 1",
    result.nextValue,
    result.operationId,
  );
}

export const registry = setup({ use: { workflowCounter } });
