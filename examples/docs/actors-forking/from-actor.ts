import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export const workspace = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS branch (id INTEGER PRIMARY KEY CHECK (id = 1), label TEXT)",
			);
		},
	}),

	// Runs in the new actor and receives the input that `branch` passed to fork.
	onFork: async (c, fork) => {
		const label =
			typeof fork.input === "string" ? fork.input.slice(0, 100) : null;
		await c.db.execute(
			"INSERT OR REPLACE INTO branch (id, label) VALUES (1, ?)",
			label,
		);
	},

	actions: {
		// Forks this actor and returns the new actor's ID.
		branch: async (c, label: string): Promise<string> => {
			const client = c.client<typeof registry>();
			const copy = await client.workspace.fork(c.actorId, {
				input: label,
			});
			return await copy.resolve();
		},
	},
});

export const registry = setup({ use: { workspace } });
