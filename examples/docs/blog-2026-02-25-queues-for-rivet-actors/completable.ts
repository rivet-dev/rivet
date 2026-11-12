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
    // undefined response type means ack-only, no return data
    process: queue<{ taskId: string }, undefined>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter({ completable: true })) {
      // Do work
      await processTask(message.body.taskId);
      await c.db.execute("UPDATE stats SET processed = processed + 1 WHERE id = 1");
      // Acknowledge completion without returning data
      await message.complete();
    }
  },
});

async function processTask(taskId: string) {
  await fetch(`https://api.example.com/tasks/${taskId}/complete`, {
    method: "POST",
  });
}

export const registry = setup({ use: { worker } });
