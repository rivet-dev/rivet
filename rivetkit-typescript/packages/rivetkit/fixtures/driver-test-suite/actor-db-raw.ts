import { actor } from "rivetkit";
import { db } from "rivetkit/db";

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
		sleepTimeout: 100,
	},
});
