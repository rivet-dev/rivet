import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { Loop, type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type WorkMessage = { amount: number };
type ControlMessage = { type: "stop"; reason: string };

export const setupRunTeardownActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS worker_status (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          phase TEXT NOT NULL,
          total INTEGER NOT NULL,
          processed INTEGER NOT NULL,
          stop_reason TEXT,
          worker_session_id TEXT
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO worker_status (id, phase, total, processed, stop_reason, worker_session_id) VALUES (1, 'idle', 0, 0, NULL, NULL)",
      );
    },
  }),
  queues: {
    work: queue<WorkMessage>(),
    control: queue<ControlMessage>(),
  },
  run: workflow(async (ctx) => {
    await ctx.step("setup", async (step) => setupWorkerSession(step));

    const stopReason = await ctx.loop("worker-loop", async (loopCtx) => {
        const message = await loopCtx.queue.next("wait-command", {
          names: ["work", "control"],
        });

        if (message.name === "work") {
          const work = message.body as WorkMessage;
          await loopCtx.step("apply-work", async (step) =>
            applyWorkerMessage(step, work),
          );
          return;
        }

        const control = message.body as ControlMessage;
        if (control.type === "stop") {
          return Loop.break(control.reason);
        }

      });

    await ctx.step("teardown", async (step) =>
      teardownWorkerSession(step, stopReason),
    );
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        phase: "idle" | "running" | "stopped";
        total: number;
        processed: number;
        stopReason: string | null;
        workerSessionId: string | null;
      }>(
        "SELECT phase, total, processed, stop_reason AS stopReason, worker_session_id AS workerSessionId FROM worker_status WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function setupWorkerSession(
  ctx: WorkflowStepContextOf<typeof setupRunTeardownActor>,
): Promise<void> {
  const response = await fetch("https://api.example.com/workers/session", {
    method: "POST",
  });
  if (!response.ok) {
    throw new Error(`worker setup failed: ${response.status}`);
  }
  const session = (await response.json()) as { sessionId: string };
  await ctx.db.execute(
    "UPDATE worker_status SET worker_session_id = ?, phase = 'running', stop_reason = NULL WHERE id = 1",
    session.sessionId,
  );
}

async function getWorkerSessionId(
  ctx: WorkflowStepContextOf<typeof setupRunTeardownActor>,
): Promise<string | null> {
  const rows = await ctx.db.execute<{ workerSessionId: string | null }>(
    "SELECT worker_session_id AS workerSessionId FROM worker_status WHERE id = 1",
  );
  return rows[0].workerSessionId;
}

async function applyWorkerMessage(
  ctx: WorkflowStepContextOf<typeof setupRunTeardownActor>,
  work: WorkMessage,
): Promise<void> {
  const sessionId = await getWorkerSessionId(ctx);
  const response = await fetch("https://api.example.com/workers/process", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({
      sessionId,
      amount: work.amount,
    }),
  });
  if (!response.ok) {
    throw new Error(`worker process failed: ${response.status}`);
  }
  const result = (await response.json()) as { appliedAmount: number };
  await ctx.db.execute(
    "UPDATE worker_status SET total = total + ?, processed = processed + 1 WHERE id = 1",
    result.appliedAmount,
  );
}

async function teardownWorkerSession(
  ctx: WorkflowStepContextOf<typeof setupRunTeardownActor>,
  stopReason: string,
): Promise<void> {
  const sessionId = await getWorkerSessionId(ctx);
  if (sessionId) {
    const response = await fetch(
      `https://api.example.com/workers/session/${sessionId}`,
      { method: "DELETE" },
    );
    if (!response.ok) {
      throw new Error(`worker teardown failed: ${response.status}`);
    }
  }
  await ctx.db.execute(
    "UPDATE worker_status SET phase = 'stopped', stop_reason = ? WHERE id = 1",
    stopReason,
  );
}

export const registry = setup({ use: { setupRunTeardownActor } });
