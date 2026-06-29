import { actor, queue, setup } from "rivetkit";
import { Loop, type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

type WorkMessage = { amount: number };
type ControlMessage = { type: "stop"; reason: string };

export const setupRunTeardownActor = actor({
  state: {
    phase: "idle" as "idle" | "running" | "stopped",
    total: 0,
    processed: 0,
    stopReason: null as string | null,
    workerSessionId: null as string | null,
  },
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
    getState: (c) => c.state,
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
  ctx.state.workerSessionId = session.sessionId;
  ctx.state.phase = "running";
  ctx.state.stopReason = null;
}

async function applyWorkerMessage(
  ctx: WorkflowStepContextOf<typeof setupRunTeardownActor>,
  work: WorkMessage,
): Promise<void> {
  const response = await fetch("https://api.example.com/workers/process", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({
      sessionId: ctx.state.workerSessionId,
      amount: work.amount,
    }),
  });
  if (!response.ok) {
    throw new Error(`worker process failed: ${response.status}`);
  }
  const result = (await response.json()) as { appliedAmount: number };
  ctx.state.total += result.appliedAmount;
  ctx.state.processed += 1;
}

async function teardownWorkerSession(
  ctx: WorkflowStepContextOf<typeof setupRunTeardownActor>,
  stopReason: string,
): Promise<void> {
  if (ctx.state.workerSessionId) {
    const response = await fetch(
      `https://api.example.com/workers/session/${ctx.state.workerSessionId}`,
      { method: "DELETE" },
    );
    if (!response.ok) {
      throw new Error(`worker teardown failed: ${response.status}`);
    }
  }
  ctx.state.phase = "stopped";
  ctx.state.stopReason = stopReason;
}

export const registry = setup({ use: { setupRunTeardownActor } });
