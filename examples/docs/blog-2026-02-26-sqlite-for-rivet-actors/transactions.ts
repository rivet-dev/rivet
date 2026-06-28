import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

const ledger = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS accounts (
          id TEXT PRIMARY KEY,
          balance INTEGER NOT NULL DEFAULT 0
        );
      `);
      await db.execute(`
        CREATE TABLE IF NOT EXISTS transfers (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          from_id TEXT NOT NULL,
          to_id TEXT NOT NULL,
          amount INTEGER NOT NULL
        );
      `);
    },
  }),
  actions: {
    // Atomic transfer: debit, credit, and log succeed or fail together
    transfer: async (c, fromId: string, toId: string, amount: number) => {
      await c.db.execute("BEGIN");
      try {
        // Debit sender
        await c.db.execute(
          "UPDATE accounts SET balance = balance - ? WHERE id = ?",
          amount,
          fromId,
        );
        // Credit receiver
        await c.db.execute(
          "UPDATE accounts SET balance = balance + ? WHERE id = ?",
          amount,
          toId,
        );
        // Log the transfer
        await c.db.execute(
          "INSERT INTO transfers (from_id, to_id, amount) VALUES (?, ?, ?)",
          fromId,
          toId,
          amount,
        );
        await c.db.execute("COMMIT");
      } catch (error) {
        // Roll back all changes on failure
        await c.db.execute("ROLLBACK");
        throw error;
      }
    },
    getBalance: async (c, accountId: string) => {
      const rows = (await c.db.execute(
        "SELECT balance FROM accounts WHERE id = ?",
        accountId,
      )) as { balance: number }[];
      return rows[0]?.balance ?? 0;
    },
  },
});

export const registry = setup({ use: { ledger } });
