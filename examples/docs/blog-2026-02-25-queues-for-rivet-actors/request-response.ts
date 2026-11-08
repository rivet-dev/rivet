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
  queues: {
    // Second type parameter is the response type
    increment: queue<{ amount: number }, { value: number }>(),
  },
  run: async (c) => {
    // Enable completable to allow responding to callers
    for await (const message of c.queue.iter({ completable: true })) {
      await c.db.execute(
        "UPDATE counter SET value = value + ? WHERE id = 1",
        message.body.amount,
      );
      const rows = (await c.db.execute(
        "SELECT value FROM counter WHERE id = 1",
      )) as { value: number }[];
      // Send typed response back to the caller
      await message.complete({ value: rows[0].value });
    }
  },
});

export const registry = setup({ use: { counter } });
