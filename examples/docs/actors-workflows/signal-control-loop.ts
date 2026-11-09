import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type ControlSignal = { kind: "pause" | "resume" | "stop" };

export const controlLoopActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS control_status (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          mode TEXT NOT NULL,
          handled_signals INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO control_status (id, mode, handled_signals) VALUES (1, 'running', 0)",
      );
    },
  }),
  run: workflow(async (ctx) => {
    await ctx.loop("control-loop", async (loopCtx) => {
        const [message] = await loopCtx.queue.nextBatch("wait-signal", {
          timeout: 30_000,
        });

        if (!message) return;
        const signal = message.body as ControlSignal;

        await loopCtx.step("apply-signal", async (step) =>
          applyControlSignal(step, signal.kind),
        );

      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        mode: "running" | "paused" | "stopped";
        handledSignals: number;
      }>(
        "SELECT mode, handled_signals AS handledSignals FROM control_status WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function applyControlSignal(
  ctx: WorkflowStepContextOf<typeof controlLoopActor>,
  kind: ControlSignal["kind"],
): Promise<void> {
  const modes = {
    pause: "paused",
    resume: "running",
    stop: "stopped",
  } as const;
  await ctx.db.execute(
    "UPDATE control_status SET mode = ?, handled_signals = handled_signals + 1 WHERE id = 1",
    modes[kind],
  );
}

export const registry = setup({ use: { controlLoopActor } });
