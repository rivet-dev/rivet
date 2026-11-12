import { actor, event, setup } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export type StreamState = {
	topValues: number[];
};

export type StreamUpdate = {
	topValues: number[];
	totalCount: number;
	highestValue: number | null;
};

const streamProcessor = actor({
	// Durable data that survives restarts: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS stats (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					total_values INTEGER NOT NULL
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO stats (id, total_values) VALUES (1, 0)",
			);
			await db.execute(
				"CREATE TABLE IF NOT EXISTS top_values (value REAL NOT NULL)",
			);
		},
	}),

	events: {
		updated: event<StreamUpdate>(),
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getTopValues: async (c) => (await readStats(c.db)).topValues,

		getStats: (c) => readStats(c.db),

		addValue: async (c, value: number) => {
			// Update the count and the top values in one transaction so
			// concurrent calls never overwrite each other.
			const result = await c.db.transaction(
				async (tx) => {
					await tx.execute(
						"UPDATE stats SET total_values = total_values + 1 WHERE id = 1",
					);
					await tx.execute(
						"INSERT INTO top_values (value) VALUES (?)",
						value,
					);

					// Keep only the top 3 values.
					await tx.execute(
						"DELETE FROM top_values WHERE rowid NOT IN (SELECT rowid FROM top_values ORDER BY value DESC LIMIT 3)",
					);

					return await readStats(tx);
				},
				{ name: "add-value" },
			);

			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("updated", result);

			return result.topValues;
		},

		reset: async (c) => {
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						"UPDATE stats SET total_values = 0 WHERE id = 1",
					);
					await tx.execute("DELETE FROM top_values");
				},
				{ name: "reset-stats" },
			);

			const result: StreamUpdate = {
				topValues: [],
				totalCount: 0,
				highestValue: null,
			};

			c.broadcast("updated", result);

			return result;
		},
	},
});

async function readStats(db: RawAccess): Promise<StreamUpdate> {
	const [{ totalCount }] = await db.execute<{ totalCount: number }>(
		"SELECT total_values AS totalCount FROM stats WHERE id = 1",
	);
	const rows = await db.execute<{ value: number }>(
		"SELECT value FROM top_values ORDER BY value DESC",
	);
	const topValues = rows.map((row) => row.value);
	return {
		topValues,
		totalCount,
		highestValue: topValues.length > 0 ? topValues[0] : null,
	};
}

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { streamProcessor },
});

registry.start();
