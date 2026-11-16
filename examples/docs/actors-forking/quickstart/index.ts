import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { z } from "zod";

// Fork input comes from whoever calls fork, so validate it like a request body.
const ForkInput = z.object({ title: z.string().min(1).max(200) });

export const document = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS document (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					title TEXT NOT NULL,
					body TEXT NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO document VALUES (1, 'Untitled', '')",
			);
		},
	}),

	// Runs once, at the first start of a fork, instead of onCreate.
	onFork: async (c, fork) => {
		const input = ForkInput.safeParse(fork.input);
		if (input.success) {
			await c.db.execute(
				"UPDATE document SET title = ? WHERE id = 1",
				input.data.title,
			);
		}
	},

	actions: {
		write: async (c, body: string) => {
			await c.db.execute(
				"UPDATE document SET body = ? WHERE id = 1",
				body,
			);
		},
		read: async (c) => {
			const [doc] = await c.db.execute<{ title: string; body: string }>(
				"SELECT title, body FROM document WHERE id = 1",
			);
			// The actor this one was forked from, or null for an original.
			const forkedFrom = c.forkChain.at(-1)?.actorId ?? null;
			return { title: doc.title, body: doc.body, forkedFrom };
		},
	},
});

export const registry = setup({ use: { document } });

registry.start();
