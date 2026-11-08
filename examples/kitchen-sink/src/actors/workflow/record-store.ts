// RECORD STORE
// Each workflow demo actor owns one JSON record that its workflow steps
// update. The record lives in a single-row SQLite table so it survives sleep.
// Change it with `updateRecord`, which reads and writes it in one transaction,
// so concurrent steps and actions never overwrite each other's changes.
// Workflow code outside a step re-runs on every replay, so only update the
// record inside a step or an action.

import { db, type RawAccess } from "rivetkit/db";

/** Creates a database whose `record` table holds the actor's single JSON record. */
export function recordDb() {
	return db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS record (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					data TEXT NOT NULL
				)
			`);
		},
	});
}

/**
 * Writes the record, replacing any previous version. Use it in `onCreate`, or
 * inside a transaction that loaded the record first.
 */
export async function saveRecord<T>(
	database: RawAccess,
	record: T,
): Promise<void> {
	await database.execute(
		"INSERT OR REPLACE INTO record (id, data) VALUES (1, ?)",
		JSON.stringify(record),
	);
}

/** Reads the record written by `onCreate`. Throws if it was never written. */
export async function loadRecord<T>(database: RawAccess): Promise<T> {
	const rows = await database.execute<{ data: string }>(
		"SELECT data FROM record WHERE id = 1",
	);
	const row = rows[0];
	if (!row) {
		throw new Error("actor record is missing; onCreate must write it");
	}
	return JSON.parse(row.data) as T;
}

/** Applies `change` to the record in one transaction and returns the saved record. */
export async function updateRecord<T>(
	database: RawAccess,
	change: (record: T) => void,
): Promise<T> {
	return await database.transaction(
		async (tx) => {
			const record = await loadRecord<T>(tx);
			change(record);
			await saveRecord(tx, record);
			return record;
		},
		{ name: "update-record" },
	);
}
