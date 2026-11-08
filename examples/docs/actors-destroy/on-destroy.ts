import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface UserInput {
  email: string;
  name: string;
}

// Example email service interface
const emailService = {
  send: async (options: { from: string; to: string; subject: string; text: string }) => {},
};

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
  onDestroy: async (c) => {
    const rows = (await c.db.execute(
      "SELECT email, name FROM profile WHERE id = 1",
    )) as { email: string; name: string }[];
    const user = rows[0];

    await emailService.send({
      from: "noreply@example.com",
      to: user.email,
      subject: "Account Deleted",
      text: `Goodbye ${user.name}, your account has been deleted.`,
    });
  },
  actions: {
    deleteAccount: (c) => {
      c.destroy();
    },
  },
});
