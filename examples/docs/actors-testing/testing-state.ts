import { test, expect } from "vitest";
import { setupTest } from "rivetkit/test";
import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// Define the counter actor
const counter = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
      );
      await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
    },
  }),
  actions: {
    increment: async (c) => {
      await c.db.execute("UPDATE counter SET count = count + 1 WHERE id = 1");
      const [{ count }] = (await c.db.execute(
        "SELECT count FROM counter WHERE id = 1",
      )) as { count: number }[];
      c.broadcast("newCount", count);
      return count;
    },
    getCount: async (c) => {
      const [{ count }] = (await c.db.execute(
        "SELECT count FROM counter WHERE id = 1",
      )) as { count: number }[];
      return count;
    }
  }
});

// Create the registry
const registry = setup({
  use: { counter }
});

// Test data persistence
test("actor should persist data", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);
  const counterHandle = client.counter.getOrCreate(["test"]);

  // Initial value
  expect(await counterHandle.getCount()).toBe(0);

  // Modify the value
  await counterHandle.increment();

  // Verify the value was updated
  expect(await counterHandle.getCount()).toBe(1);
});
