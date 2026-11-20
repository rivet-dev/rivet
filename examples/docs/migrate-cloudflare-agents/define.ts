import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

interface AgentInput {
	ownerId: string;
}

export const supportAgent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS settings (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          owner_id TEXT NOT NULL
        )
      `);
		},
	}),

	// Runs once, when the actor is first created
	onCreate: async (c, input: AgentInput) => {
		await c.db.execute(
			"INSERT INTO settings (id, owner_id) VALUES (1, ?)",
			input.ownerId,
		);
	},

	// Runs on every start, like onStart
	onWake: (c) => {
		console.log("agent started", c.key);
	},

	// Every action can be called by clients, like a @callable() method
	actions: {
		greet: (_c, name: string) => `Hello, ${name}!`,

		owner: async (c) => {
			const rows = await c.db.execute<{ owner_id: string }>(
				"SELECT owner_id FROM settings WHERE id = 1",
			);
			return rows[0]?.owner_id;
		},
	},
});

export const registry = setup({ use: { supportAgent } });
registry.start();
