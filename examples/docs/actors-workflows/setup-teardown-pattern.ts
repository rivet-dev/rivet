import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

function openResource(): string {
  return "connected";
}

function closeResource(_resource: string): void {}

export const setupRunTeardownActor = actor({
  vars: {
    resource: null as string | null,
  },
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS worker_status (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          initialized INTEGER NOT NULL,
          ticks INTEGER NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO worker_status (id, initialized, ticks) VALUES (1, 0, 0)",
      );
    },
  }),
  onWake: (c) => {
    c.vars.resource = openResource();
  },
  onSleep: (c) => {
    if (!c.vars.resource) return;
    closeResource(c.vars.resource);
    c.vars.resource = null;
  },
  run: workflow(async (ctx) => {
    await ctx.step("setup", async (step) => {
      if (!step.vars.resource) step.vars.resource = openResource();
      await step.db.execute("UPDATE worker_status SET initialized = 1 WHERE id = 1");
    });

    await ctx.loop("main-loop", async (loopCtx) => {
        await loopCtx.sleep("tick", 1_000);
        await loopCtx.step("tick-step", async (step) => {
          await step.db.execute("UPDATE worker_status SET ticks = ticks + 1 WHERE id = 1");
        });
      });
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ initialized: number; ticks: number }>(
        "SELECT initialized, ticks FROM worker_status WHERE id = 1",
      );
      return { initialized: rows[0].initialized === 1, ticks: rows[0].ticks };
    },
  },
});

export const registry = setup({ use: { setupRunTeardownActor } });
