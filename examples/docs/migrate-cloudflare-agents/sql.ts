import { actor } from "rivetkit";
import { db } from "rivetkit/db";

type User = {
	id: string;
	name: string;
};

export const directory = actor({
	// Runs on every start, before any other hook touches the database
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
			);
		},
	}),

	actions: {
		getUser: async (c, id: string) => {
			// Placeholders replace template literal interpolation
			const [user] = await c.db.execute<User>(
				"SELECT id, name FROM users WHERE id = ?",
				id,
			);
			return user ?? null;
		},

		importUsers: async (c, users: User[]) => {
			// All rows are written, or none are
			await c.db.transaction(async (tx) => {
				for (const user of users) {
					await tx.execute(
						"INSERT OR REPLACE INTO users (id, name) VALUES (?, ?)",
						user.id,
						user.name,
					);
				}
			});
		},
	},
});
