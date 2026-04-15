import { actor, setup } from "rivetkit";
import { db, type SqliteVfsTelemetry } from "rivetkit/db";

export const todoList = actor({
	options: {
		actionTimeout: 300_000,
	},
	db: db({
		onMigrate: async (db) => {
			// Run migrations on wake
			await db.execute(`
				CREATE TABLE IF NOT EXISTS todos (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					title TEXT NOT NULL,
					completed INTEGER DEFAULT 0,
					created_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS payload_bench (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					label TEXT NOT NULL,
					payload TEXT NOT NULL,
					payload_bytes INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await db.execute(
				"CREATE INDEX IF NOT EXISTS idx_payload_bench_label ON payload_bench(label)",
			);
		},
	}),
	actions: {
		addTodo: async (c, title: string) => {
			const createdAt = Date.now();
			await c.db.execute(
				"INSERT INTO todos (title, created_at) VALUES (?, ?)",
				title,
				createdAt,
			);
			return { title, createdAt };
		},
		getTodos: async (c) => {
			const rows = await c.db.execute("SELECT * FROM todos ORDER BY created_at DESC");
			return rows;
		},
		toggleTodo: async (c, id: number) => {
			await c.db.execute(
				"UPDATE todos SET completed = NOT completed WHERE id = ?",
				id,
			);
			const rows = await c.db.execute("SELECT * FROM todos WHERE id = ?", id);
			return rows[0];
		},
		deleteTodo: async (c, id: number) => {
			await c.db.execute("DELETE FROM todos WHERE id = ?", id);
			return { id };
		},
		benchInsertPayload: async (
			c,
			label: string,
			payloadBytes: number,
			rowCount: number = 1,
		) => {
			if (!c.db.resetVfsTelemetry || !c.db.snapshotVfsTelemetry) {
				throw new Error("native SQLite VFS telemetry is unavailable");
			}

			await c.db.resetVfsTelemetry();
			const payload = "x".repeat(payloadBytes);
			const createdAt = Date.now();
			const insertStart = performance.now();

			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO payload_bench (label, payload, payload_bytes, created_at) VALUES (?, ?, ?, ?)",
					label,
					payload,
					payloadBytes,
					createdAt + i,
				);
			}
			await c.db.execute("COMMIT");

			const insertElapsedMs = performance.now() - insertStart;
			const verifyStart = performance.now();
			const [{ totalBytes, storedRows }] = (await c.db.execute(
				"SELECT COALESCE(SUM(payload_bytes), 0) as totalBytes, COUNT(*) as storedRows FROM payload_bench WHERE label = ?",
				label,
			)) as { totalBytes: number; storedRows: number }[];
			const verifyElapsedMs = performance.now() - verifyStart;
			const vfsTelemetry: SqliteVfsTelemetry =
				await c.db.snapshotVfsTelemetry();

			return {
				label,
				payloadBytes,
				rowCount,
				totalBytes,
				storedRows,
				insertElapsedMs,
				verifyElapsedMs,
				vfsTelemetry,
			};
		},
	},
});

export const registry = setup({
	use: { todoList },
});

registry.start();
