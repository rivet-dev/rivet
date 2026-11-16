import { actor } from "rivetkit";
import { db } from "rivetkit/db";

// Stands in for your provider's API.
declare function createSandbox(): Promise<string>;

const agent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS sandbox (
					id TEXT NOT NULL,
					namespace TEXT NOT NULL
				);
			`);
		},
	}),
	onWake: async (c) => {
		const rows = (await c.db.execute(
			"SELECT id, namespace FROM sandbox",
		)) as { id: string; namespace: string }[];

		// A namespace fork copies this table with the actor. A sandbox recorded
		// in another namespace belongs to the source actor, so create our own.
		if (rows[0]?.namespace !== c.namespace) {
			const id = await createSandbox();
			await c.db.execute("DELETE FROM sandbox");
			await c.db.execute(
				"INSERT INTO sandbox (id, namespace) VALUES (?, ?)",
				id,
				c.namespace,
			);
		}
	},
});
