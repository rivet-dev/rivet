import { actor, UserError } from "rivetkit";
import { db } from "rivetkit/db";

const wallet = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS wallet (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          balance INTEGER NOT NULL
        );
      `);
			await db.execute(`
        CREATE TABLE IF NOT EXISTS ledger (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          amount INTEGER NOT NULL,
          created_at INTEGER NOT NULL
        );
      `);
			await db.execute(
				"INSERT OR IGNORE INTO wallet (id, balance) VALUES (1, 0)",
			);
		},
	}),

	actions: {
		balance: async (c) => {
			const [{ balance }] = await c.db.execute<{ balance: number }>(
				"SELECT balance FROM wallet WHERE id = 1",
			);
			return balance;
		},

		// The check and the change run in one statement, so two withdrawals
		// can never both pass the check
		withdraw: async (c, amount: number) => {
			const rows = await c.db.execute<{ balance: number }>(
				"UPDATE wallet SET balance = balance - ? WHERE id = 1 AND balance >= ? RETURNING balance",
				amount,
				amount,
			);
			if (rows.length === 0) {
				throw new UserError("Insufficient balance", {
					code: "insufficient_balance",
				});
			}
			return rows[0].balance;
		},

		// Statements that must commit together run in one transaction
		deposit: async (c, amount: number) => {
			return await c.db.transaction(
				async (tx) => {
					await tx.execute(
						"INSERT INTO ledger (amount, created_at) VALUES (?, ?)",
						amount,
						Date.now(),
					);
					const [{ balance }] = await tx.execute<{ balance: number }>(
						"UPDATE wallet SET balance = balance + ? WHERE id = 1 RETURNING balance",
						amount,
					);
					return balance;
				},
				{ name: "deposit" },
			);
		},
	},
});
