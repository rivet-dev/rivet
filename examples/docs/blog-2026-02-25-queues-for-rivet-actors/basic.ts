import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";

const counter = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), value INTEGER NOT NULL)",
      );
      await db.execute("INSERT OR IGNORE INTO counter (id, value) VALUES (1, 0)");
    },
  }),
  // Define typed queues
  queues: {
    increment: queue<{ amount: number }>(),
  },
  // Process messages in the run handler
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      await c.db.execute(
        "UPDATE counter SET value = value + ? WHERE id = 1",
        message.body.amount,
      );
    }
  },
  actions: {
    getValue: async (c) => {
      const rows = (await c.db.execute(
        "SELECT value FROM counter WHERE id = 1",
      )) as { value: number }[];
      return rows[0].value;
    },
  },
});

export const registry = setup({ use: { counter } });
