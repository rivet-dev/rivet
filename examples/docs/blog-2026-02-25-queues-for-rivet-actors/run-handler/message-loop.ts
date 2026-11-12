import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";

const worker = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS stats (id INTEGER PRIMARY KEY CHECK (id = 1), processed INTEGER NOT NULL)",
      );
      await db.execute("INSERT OR IGNORE INTO stats (id, processed) VALUES (1, 0)");
    },
  }),
  queues: {
    jobs: queue<{ url: string }>(),
  },
  // Iterate messages as they arrive
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      await fetch(message.body.url, { method: "POST" });
      await c.db.execute("UPDATE stats SET processed = processed + 1 WHERE id = 1");
    }
  },
});

export const registry = setup({ use: { worker } });
