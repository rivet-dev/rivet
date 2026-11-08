import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { workflow, type WorkflowErrorEvent } from "rivetkit/workflow";

export const errorHookActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS workflow_errors (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          event TEXT NOT NULL
        );
      `);
    },
  }),
  events: {
    workflowError: event<[WorkflowErrorEvent]>(),
  },
  run: workflow(
    async (ctx) => {
      await ctx.step({
        name: "sync-ledger",
        maxRetries: 3,
        retryBackoffBase: 250,
        retryBackoffMax: 1_000,
        run: async (ctx) => {
          throw new Error("ledger unavailable");
        },
      });
    },
    {
      onError: async (c, event) => {
        await c.db.execute(
          "INSERT OR REPLACE INTO workflow_errors (id, event) VALUES (1, ?)",
          JSON.stringify(event),
        );
        c.broadcast("workflowError", event);
      },
    },
  ),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{ event: string }>(
        "SELECT event FROM workflow_errors WHERE id = 1",
      );
      const lastError = rows[0]
        ? (JSON.parse(rows[0].event) as WorkflowErrorEvent)
        : null;
      return { lastError };
    },
  },
});

export const registry = setup({ use: { errorHookActor } });
