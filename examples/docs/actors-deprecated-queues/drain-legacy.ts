import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const mailer = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS jobs (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          email TEXT NOT NULL
        )
      `);
    },
  }),
  onWake: async (c) => {
    // Move messages still waiting in the old queue into the jobs table.
    // Remove this hook once every actor has woken on the new version.
    while (true) {
      const messages = await c.queue.tryNextBatch({ count: 100 });
      if (messages.length === 0) break;

      for (const message of messages) {
        const body = message.body as { email: string };
        await c.db.execute(
          "INSERT INTO jobs (email) VALUES (?)",
          body.email,
        );
      }
    }
  },
  actions: {},
});
