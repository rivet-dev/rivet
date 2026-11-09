import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const userAccount = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS account (id INTEGER PRIMARY KEY CHECK (id = 1), email TEXT NOT NULL, name TEXT NOT NULL)",
			);
		},
	}),
	onDestroy: async (c) => {
		const rows = (await c.db.execute("SELECT email FROM account WHERE id = 1")) as {
			email: string;
		}[];
		console.log(`Account ${rows[0]?.email} deleted`);
	},
	actions: {
		deleteAccount: (c) => {
			c.destroy();
		},
	},
});
