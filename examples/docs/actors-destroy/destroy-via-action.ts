import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface UserInput {
  email: string;
  name: string;
}

const userActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS profile (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          email TEXT NOT NULL,
          name TEXT NOT NULL
        );
      `);
    },
  }),
  onCreate: async (c, input: UserInput) => {
    await c.db.execute(
      "INSERT INTO profile (id, email, name) VALUES (1, ?, ?)",
      input.email,
      input.name,
    );
  },
  actions: {
    deleteAccount: (c) => {
      c.destroy();
    },
  },
});
