import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// Mock external database interface for demonstration
const externalDb = {
  users: {
    insert: async (data: { id: string; email: string; createdAt: number }) => {},
    update: async (id: string, data: { email: string; lastActive: number }) => {},
  },
};

const userActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS profile (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          email TEXT NOT NULL,
          last_active INTEGER NOT NULL
        );
      `);
    },
  }),

  onCreate: async (c, input: { email: string }) => {
    await c.db.execute(
      "INSERT INTO profile (id, email, last_active) VALUES (1, ?, 0)",
      input.email,
    );

    // Insert into the external database on actor creation
    await externalDb.users.insert({
      id: c.key.join("-"),
      email: input.email,
      createdAt: Date.now(),
    });
  },

  actions: {
    updateEmail: async (c, email: string) => {
      const lastActive = Date.now();
      await c.db.execute(
        "UPDATE profile SET email = ?, last_active = ? WHERE id = 1",
        email,
        lastActive,
      );

      // Sync the change to the external database right after writing it
      await externalDb.users.update(c.key.join("-"), { email, lastActive });
    },
    getUser: async (c) => {
      const rows = (await c.db.execute(
        "SELECT email, last_active FROM profile WHERE id = 1",
      )) as { email: string; last_active: number }[];
      return { email: rows[0].email, lastActive: rows[0].last_active };
    },
  },
});

const registry = setup({
  use: { userActor },
});
