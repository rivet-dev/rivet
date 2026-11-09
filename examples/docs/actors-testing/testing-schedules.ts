import { expect, test } from "vitest";
import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { setupTest } from "rivetkit/test";

const scheduler = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS completed_tasks (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
      );
    },
  }),
  actions: {
    scheduleTask: async (c, taskName: string) => {
      await c.schedule.after(50, "completeTask", taskName);
    },
    completeTask: async (c, taskName: string) => {
      await c.db.execute("INSERT INTO completed_tasks (name) VALUES (?)", taskName);
    },
    getCompletedTasks: async (c) => {
      const rows = (await c.db.execute(
        "SELECT name FROM completed_tasks ORDER BY id",
      )) as { name: string }[];
      return rows.map((row) => row.name);
    },
  },
});

const registry = setup({ use: { scheduler } });

test("scheduled work updates observable state", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);
  const handle = client.scheduler.getOrCreate(["test"]);

  await handle.scheduleTask("reminder");

  await expect
    .poll(() => handle.getCompletedTasks(), { timeout: 2_000, interval: 25 })
    .toContain("reminder");
});
