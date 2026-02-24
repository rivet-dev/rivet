import { actor } from "rivetkit";
import { db } from "rivetkit/db";

function firstRowValue(row: Record<string, unknown> | undefined): unknown {
	if (!row) {
		return undefined;
	}

	const values = Object.values(row);
	return values.length > 0 ? values[0] : undefined;
}

function toSafeInteger(value: unknown): number {
	if (typeof value === "bigint") {
		return Number(value);
	}
	if (typeof value === "number") {
		return Number.isFinite(value) ? Math.trunc(value) : 0;
	}
	if (typeof value === "string") {
		const parsed = Number.parseInt(value, 10);
		return Number.isFinite(parsed) ? parsed : 0;
	}
	return 0;
}

function normalizeRowIds(rowIds: number[]): number[] {
	const normalized = rowIds
		.map((id) => Math.trunc(id))
		.filter((id) => Number.isFinite(id) && id > 0);
	return Array.from(new Set(normalized));
}

function makePayload(size: number): string {
	const normalizedSize = Math.max(0, Math.trunc(size));
	return "x".repeat(normalizedSize);
}

export const dbActorRaw = actor({
	state: {
		disconnectInsertEnabled: false,
		disconnectInsertDelayMs: 0,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS test_data (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					value TEXT NOT NULL,
					payload TEXT NOT NULL DEFAULT '',
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onDisconnect: async (c) => {
		if (!c.state.disconnectInsertEnabled) {
			return;
		}

		if (c.state.disconnectInsertDelayMs > 0) {
			await new Promise<void>((resolve) =>
				setTimeout(resolve, c.state.disconnectInsertDelayMs),
			);
		}

		await c.db.execute(
			`INSERT INTO test_data (value, payload, created_at) VALUES ('__disconnect__', '', ${Date.now()})`,
		);
	},
	actions: {
		configureDisconnectInsert: (c, enabled: boolean, delayMs: number) => {
			c.state.disconnectInsertEnabled = enabled;
			c.state.disconnectInsertDelayMs = Math.max(
				0,
				Math.floor(delayMs),
			);
		},
		getDisconnectInsertCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM test_data WHERE value = '__disconnect__'`,
			);
			return results[0]?.count ?? 0;
		},
		reset: async (c) => {
			await c.db.execute(`DELETE FROM test_data`);
		},
		insertValue: async (c, value: string) => {
			await c.db.execute(
				`INSERT INTO test_data (value, payload, created_at) VALUES ('${value}', '', ${Date.now()})`,
			);
			const results = await c.db.execute<{ id: number }>(
				`SELECT last_insert_rowid() as id`,
			);
			return { id: results[0].id };
		},
		getValues: async (c) => {
			const results = await c.db.execute<{
				id: number;
				value: string;
				payload: string;
				created_at: number;
			}>(
				`SELECT * FROM test_data ORDER BY id`,
			);
			return results;
		},
		getValue: async (c, id: number) => {
			const results = await c.db.execute<{ value: string }>(
				`SELECT value FROM test_data WHERE id = ${id}`,
			);
			return results[0]?.value ?? null;
		},
		getCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM test_data`,
			);
			return results[0].count;
		},
		rawSelectCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM test_data`,
			);
			return results[0].count;
		},
		insertMany: async (c, count: number) => {
			if (count <= 0) {
				return { count: 0 };
			}
			const now = Date.now();
			const values: string[] = [];
			for (let i = 0; i < count; i++) {
				values.push(`('User ${i}', '', ${now})`);
			}
			await c.db.execute(
				`INSERT INTO test_data (value, payload, created_at) VALUES ${values.join(", ")}`,
			);
			return { count };
		},
		updateValue: async (c, id: number, value: string) => {
			await c.db.execute(
				`UPDATE test_data SET value = '${value}' WHERE id = ${id}`,
			);
			return { success: true };
		},
		deleteValue: async (c, id: number) => {
			await c.db.execute(`DELETE FROM test_data WHERE id = ${id}`);
		},
		transactionCommit: async (c, value: string) => {
			await c.db.execute(
				`BEGIN; INSERT INTO test_data (value, payload, created_at) VALUES ('${value}', '', ${Date.now()}); COMMIT;`,
			);
		},
		transactionRollback: async (c, value: string) => {
			await c.db.execute(
				`BEGIN; INSERT INTO test_data (value, payload, created_at) VALUES ('${value}', '', ${Date.now()}); ROLLBACK;`,
			);
		},
		insertPayloadOfSize: async (c, size: number) => {
			const payload = "x".repeat(size);
			await c.db.execute(
				`INSERT INTO test_data (value, payload, created_at) VALUES ('payload', '${payload}', ${Date.now()})`,
			);
			const results = await c.db.execute<{ id: number }>(
				`SELECT last_insert_rowid() as id`,
			);
			return { id: results[0].id, size };
		},
		getPayloadSize: async (c, id: number) => {
			const results = await c.db.execute<{ size: number }>(
				`SELECT length(payload) as size FROM test_data WHERE id = ${id}`,
			);
			return results[0]?.size ?? 0;
		},
		insertPayloadRows: async (c, count: number, payloadSize: number) => {
			const normalizedCount = Math.max(0, Math.trunc(count));
			if (normalizedCount === 0) {
				return { count: 0 };
			}

			const payload = makePayload(payloadSize);
			const now = Date.now();
			for (let i = 0; i < normalizedCount; i++) {
				await c.db.execute(
					`INSERT INTO test_data (value, payload, created_at) VALUES ('bulk-${i}', '${payload}', ${now})`,
				);
			}

			return { count: normalizedCount };
		},
		roundRobinUpdateValues: async (
			c,
			rowIds: number[],
			iterations: number,
		) => {
			const normalizedRowIds = normalizeRowIds(rowIds);
			const normalizedIterations = Math.max(0, Math.trunc(iterations));
			if (normalizedRowIds.length === 0 || normalizedIterations === 0) {
				const emptyRows: Array<{ id: number; value: string }> = [];
				return emptyRows;
			}

			for (let i = 0; i < normalizedIterations; i++) {
				const rowId = normalizedRowIds[i % normalizedRowIds.length] ?? 0;
				await c.db.execute(
					`UPDATE test_data SET value = 'v-${i}' WHERE id = ${rowId}`,
				);
			}

			return await c.db.execute<{ id: number; value: string }>(
				`SELECT id, value FROM test_data WHERE id IN (${normalizedRowIds.join(",")}) ORDER BY id`,
			);
		},
		getPageCount: async (c) => {
			const rows = await c.db.execute<Record<string, unknown>>(
				"PRAGMA page_count",
			);
			return toSafeInteger(firstRowValue(rows[0]));
		},
		vacuum: async (c) => {
			await c.db.execute("VACUUM");
		},
		integrityCheck: async (c) => {
			const rows = await c.db.execute<Record<string, unknown>>(
				"PRAGMA integrity_check",
			);
			const value = firstRowValue(rows[0]);
			return String(value ?? "");
		},
		runMixedWorkload: async (c, seedCount: number, churnCount: number) => {
			const normalizedSeedCount = Math.max(1, Math.trunc(seedCount));
			const normalizedChurnCount = Math.max(0, Math.trunc(churnCount));
			const now = Date.now();

			for (let i = 0; i < normalizedSeedCount; i++) {
				const payload = makePayload(1024 + (i % 5) * 128);
				await c.db.execute(
					`INSERT OR REPLACE INTO test_data (id, value, payload, created_at) VALUES (${i + 1}, 'seed-${i}', '${payload}', ${now})`,
				);
			}

			for (let i = 0; i < normalizedChurnCount; i++) {
				const id = (i % normalizedSeedCount) + 1;
				if (i % 9 === 0) {
					await c.db.execute(`DELETE FROM test_data WHERE id = ${id}`);
				} else {
					const payload = makePayload(768 + (i % 7) * 96);
					await c.db.execute(
						`INSERT OR REPLACE INTO test_data (id, value, payload, created_at) VALUES (${id}, 'upd-${i}', '${payload}', ${now + i})`,
					);
				}
			}
		},
		repeatUpdate: async (c, id: number, count: number) => {
			let value = "";
			if (count <= 0) {
				return { value };
			}
			const statements: string[] = ["BEGIN"];
			for (let i = 0; i < count; i++) {
				value = `Updated ${i}`;
				statements.push(
					`UPDATE test_data SET value = '${value}' WHERE id = ${id}`,
				);
			}
			statements.push("COMMIT");
			await c.db.execute(statements.join("; "));
			return { value };
		},
		multiStatementInsert: async (c, value: string) => {
			await c.db.execute(
				`BEGIN; INSERT INTO test_data (value, payload, created_at) VALUES ('${value}', '', ${Date.now()}); UPDATE test_data SET value = '${value}-updated' WHERE id = last_insert_rowid(); COMMIT;`,
			);
			const results = await c.db.execute<{ value: string }>(
				`SELECT value FROM test_data ORDER BY id DESC LIMIT 1`,
			);
			return results[0]?.value ?? null;
		},
		triggerSleep: (c) => {
			c.sleep();
		},
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});
