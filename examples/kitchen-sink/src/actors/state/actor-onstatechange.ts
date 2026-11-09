import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export const onStateChangeActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS value (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					value INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO value (id, value) VALUES (1, 0)",
			);
		},
	}),
	// The change count only describes the current run, so it lives in memory.
	vars: {
		changeCount: 0,
	},
	actions: {
		// Action that modifies the value and records the change inline
		setValue: async (c, newValue: number) => {
			const [row] = await c.db.execute<{ value: number }>(
				"UPDATE value SET value = ? WHERE id = 1 RETURNING value",
				newValue,
			);
			c.vars.changeCount++;
			return row.value;
		},
		// Action that adds to the value in one write and records one change
		incrementMultiple: async (c, times: number) => {
			const [row] = await c.db.execute<{ value: number }>(
				"UPDATE value SET value = value + ? WHERE id = 1 RETURNING value",
				times,
			);
			c.vars.changeCount++;
			return row.value;
		},
		// Action that doesn't modify the value, so no change is recorded
		getValue: async (c) => {
			return await loadValue(c.db);
		},
		// Action that reads and returns without modifying, so no change is recorded
		getDoubled: async (c) => {
			const doubled = (await loadValue(c.db)) * 2;
			return doubled;
		},
		// Get the count of how many changes were recorded
		getChangeCount: (c) => {
			return c.vars.changeCount;
		},
		// Reset change counter for testing
		resetChangeCount: (c) => {
			c.vars.changeCount = 0;
		},
	},
});

async function loadValue(database: RawAccess): Promise<number> {
	const [row] = await database.execute<{ value: number }>(
		"SELECT value FROM value WHERE id = 1",
	);
	return row.value;
}
