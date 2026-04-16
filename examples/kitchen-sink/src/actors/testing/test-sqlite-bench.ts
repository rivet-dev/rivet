import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const CHAT_LOG_CHUNK_BYTES = 4 * 1024;
const CHAT_LOG_INSERT_BATCH_SIZE = 50;

function buildChatLogMessage(seq: number, targetBytes: number): string {
	const prefix = `message-${seq}: `;
	return prefix + "x".repeat(Math.max(0, targetBytes - prefix.length));
}

async function seedChatLog(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
) {
	const threadId = `chat-${crypto.randomUUID()}`;
	const createdAtBase = Date.now();
	let remainingBytes = targetBytes;
	let rows = 0;

	await database.execute("BEGIN");
	try {
		while (remainingBytes > 0) {
			const placeholders: string[] = [];
			const args: unknown[] = [];

			for (
				let batchIndex = 0;
				batchIndex < CHAT_LOG_INSERT_BATCH_SIZE && remainingBytes > 0;
				batchIndex++
			) {
				const contentBytes = Math.min(CHAT_LOG_CHUNK_BYTES, remainingBytes);
				const seq = rows;
				const role = seq % 2 === 0 ? "user" : "assistant";

				placeholders.push("(?, ?, ?, ?, ?, ?, ?)");
				args.push(
					threadId,
					seq,
					role,
					buildChatLogMessage(seq, contentBytes),
					contentBytes,
					Math.ceil(contentBytes / 4),
					createdAtBase + seq,
				);

				remainingBytes -= contentBytes;
				rows++;
			}

			await database.execute(
				`INSERT INTO chat_log (thread_id, seq, role, content, content_bytes, token_estimate, created_at) VALUES ${placeholders.join(", ")}`,
				...args,
			);
		}

		await database.execute("COMMIT");
	} catch (err) {
		await database.execute("ROLLBACK");
		throw err;
	}

	return { threadId, rows, totalBytes: targetBytes };
}

export const testSqliteBench = actor({
	options: {
		actionTimeout: 300_000,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`CREATE TABLE IF NOT EXISTS bench (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				key TEXT NOT NULL,
				value TEXT NOT NULL,
				num INTEGER NOT NULL DEFAULT 0,
				payload BLOB,
				created_at INTEGER NOT NULL DEFAULT 0
			)`);
			await database.execute("CREATE INDEX IF NOT EXISTS idx_bench_key ON bench(key)");
			await database.execute("CREATE INDEX IF NOT EXISTS idx_bench_num ON bench(num)");

			await database.execute(`CREATE TABLE IF NOT EXISTS bench_json (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				data TEXT NOT NULL DEFAULT '{}'
			)`);

			await database.execute(`CREATE TABLE IF NOT EXISTS bench_secondary (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				bench_id INTEGER NOT NULL,
				label TEXT NOT NULL,
				score REAL NOT NULL DEFAULT 0,
				FOREIGN KEY (bench_id) REFERENCES bench(id)
			)`);

			await database.execute(`CREATE TABLE IF NOT EXISTS chat_log (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				thread_id TEXT NOT NULL,
				seq INTEGER NOT NULL,
				role TEXT NOT NULL,
				content TEXT NOT NULL,
				content_bytes INTEGER NOT NULL,
				token_estimate INTEGER NOT NULL,
				created_at INTEGER NOT NULL DEFAULT 0
			)`);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_chat_log_thread_seq ON chat_log(thread_id, seq DESC)",
			);
		},
	}),
	actions: {
		noop: (_c) => ({ ok: true }),

		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},

		insertSingle: async (c, n: number) => {
			const t0 = performance.now();
			for (let i = 0; i < n; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`k-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			return { ms: performance.now() - t0, ops: n };
		},

		insertTx: async (c, n: number) => {
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < n; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`k-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: n };
		},

		insertBatch: async (c, n: number) => {
			const t0 = performance.now();
			const placeholders = Array.from({ length: n }, () => "(?, ?, ?, ?)").join(", ");
			const args: unknown[] = [];
			for (let i = 0; i < n; i++) {
				args.push(`k-${i}`, `v-${i}`, i, Date.now());
			}
			await c.db.execute(`INSERT INTO bench (key, value, num, created_at) VALUES ${placeholders}`, ...args);
			return { ms: performance.now() - t0, ops: n };
		},

		pointRead: async (c, n: number) => {
			await c.db.execute("INSERT INTO bench (key, value, num, created_at) VALUES ('pr', 'pr', 0, 0)");
			const rows = await c.db.execute("SELECT id FROM bench WHERE key = 'pr' LIMIT 1");
			const id = (rows[0] as { id: number }).id;
			const t0 = performance.now();
			for (let i = 0; i < n; i++) {
				await c.db.execute("SELECT * FROM bench WHERE id = ?", id);
			}
			return { ms: performance.now() - t0, ops: n };
		},

		fullScan: async (c, seedRows: number) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < seedRows; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`scan-${i}`, `val-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute("SELECT * FROM bench");
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		rangeScanIndexed: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`rs-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute("SELECT * FROM bench WHERE num BETWEEN 100 AND 300");
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		rangeScanUnindexed: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`ru-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute("SELECT * FROM bench WHERE value BETWEEN 'v-100' AND 'v-300'");
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		bulkUpdate: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`bu-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			await c.db.execute("UPDATE bench SET value = 'updated', num = num + 1000 WHERE key LIKE 'bu-%'");
			return { ms: performance.now() - t0, seedMs, ops: 200 };
		},

		bulkDelete: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`bd-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			await c.db.execute("DELETE FROM bench WHERE key LIKE 'bd-%'");
			return { ms: performance.now() - t0, seedMs, ops: 200 };
		},

		hotRowUpdates: async (c, n: number) => {
			await c.db.execute("INSERT INTO bench (key, value, num, created_at) VALUES ('hot', 'v', 0, 0)");
			const rows = await c.db.execute("SELECT id FROM bench WHERE key = 'hot' LIMIT 1");
			const id = (rows[0] as { id: number }).id;
			const t0 = performance.now();
			for (let i = 0; i < n; i++) {
				await c.db.execute("UPDATE bench SET num = ? WHERE id = ?", i, id);
			}
			return { ms: performance.now() - t0, ops: n };
		},

		vacuumAfterDelete: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`vac-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			await c.db.execute("DELETE FROM bench WHERE key LIKE 'vac-%'");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			await c.db.execute("VACUUM");
			return { ms: performance.now() - t0, seedMs };
		},

		largePayloadInsert: async (c, n: number) => {
			const blob = "x".repeat(32 * 1024);
			const t0 = performance.now();
			for (let i = 0; i < n; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, payload, created_at) VALUES (?, ?, ?, ?, ?)",
					`lp-${i}`, `v-${i}`, i, blob, Date.now(),
				);
			}
			return { ms: performance.now() - t0, ops: n };
		},

		mixedOltp: async (c) => {
			const t0 = performance.now();
			await c.db.execute(
				"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
				"oltp", "initial", 0, Date.now(),
			);
			const rows = await c.db.execute("SELECT * FROM bench WHERE key = 'oltp' LIMIT 1");
			const id = (rows[0] as { id: number }).id;
			await c.db.execute("UPDATE bench SET value = 'updated', num = 1 WHERE id = ?", id);
			await c.db.execute("SELECT * FROM bench WHERE id = ?", id);
			return { ms: performance.now() - t0, ops: 4 };
		},

		jsonInsertAndQuery: async (c) => {
			const t0 = performance.now();
			for (let i = 0; i < 50; i++) {
				await c.db.execute(
					"INSERT INTO bench_json (data) VALUES (?)",
					JSON.stringify({ name: `item-${i}`, tags: ["a", "b"], score: Math.random() * 100 }),
				);
			}
			const rows = await c.db.execute(
				"SELECT id, json_extract(data, '$.name') as name, json_extract(data, '$.score') as score FROM bench_json ORDER BY json_extract(data, '$.score') DESC LIMIT 10",
			);
			return { ms: performance.now() - t0, ops: 51, rows: (rows as unknown[]).length };
		},

		jsonEachAgg: async (c) => {
			await c.db.execute(
				"INSERT INTO bench_json (data) VALUES (?)",
				JSON.stringify({ items: Array.from({ length: 100 }, (_, i) => ({ id: i, val: i * 10 })) }),
			);
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT SUM(json_extract(value, '$.val')) as total FROM bench_json, json_each(json_extract(data, '$.items')) LIMIT 1",
			);
			return { ms: performance.now() - t0, total: (rows[0] as { total: number }).total };
		},

		complexAggregation: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`grp-${i % 10}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT key, COUNT(*) as cnt, AVG(num) as avg_num, MIN(num) as min_num, MAX(num) as max_num FROM bench WHERE key LIKE 'grp-%' GROUP BY key ORDER BY cnt DESC",
			);
			return { ms: performance.now() - t0, seedMs, groups: (rows as unknown[]).length };
		},

		complexSubquery: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`sq-${i}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT * FROM bench WHERE num > (SELECT AVG(num) FROM bench) ORDER BY num DESC LIMIT 50",
			);
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		complexJoin: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`j-${i}`, `v-${i}`, i, Date.now(),
				);
				await c.db.execute(
					"INSERT INTO bench_secondary (bench_id, label, score) VALUES (?, ?, ?)",
					i + 1, `label-${i}`, Math.random() * 100,
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT b.key, b.num, s.label, s.score FROM bench b INNER JOIN bench_secondary s ON s.bench_id = b.id WHERE b.key LIKE 'j-%' ORDER BY s.score DESC LIMIT 200",
			);
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		complexCteWindow: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`cte-${i % 10}`, `v-${i}`, i, Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute(`
				WITH ranked AS (
					SELECT key, num, ROW_NUMBER() OVER (PARTITION BY key ORDER BY num DESC) as rn,
					       AVG(num) OVER (PARTITION BY key) as avg_num
					FROM bench
					WHERE key LIKE 'cte-%'
				)
				SELECT * FROM ranked WHERE rn <= 3 ORDER BY key, rn
			`);
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		migrationTables: async (c, n: number) => {
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < n; i++) {
				await c.db.execute(`CREATE TABLE IF NOT EXISTS mig_${i} (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					data TEXT NOT NULL DEFAULT ''
				)`);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: n };
		},

		chatLogInsert: async (c, totalBytes: number) => {
			const t0 = performance.now();
			const seeded = await seedChatLog(c.db, totalBytes);
			return { ms: performance.now() - t0, ops: seeded.rows, bytes: seeded.totalBytes };
		},

		chatLogSelectLimit: async (c, totalBytes: number) => {
			const seeded = await seedChatLog(c.db, totalBytes);
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT seq, role, substr(content, 1, 128) AS preview FROM chat_log ORDER BY created_at DESC LIMIT 100",
			);
			return {
				ms: performance.now() - t0,
				ops: (rows as unknown[]).length,
				rows: (rows as unknown[]).length,
				bytes: seeded.totalBytes,
			};
		},

		chatLogSelectIndexed: async (c, totalBytes: number) => {
			const seeded = await seedChatLog(c.db, totalBytes);
			const lowerBound = Math.max(0, seeded.rows - 100);
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT seq, role, content_bytes FROM chat_log WHERE thread_id = ? AND seq >= ? ORDER BY seq DESC LIMIT 100",
				seeded.threadId,
				lowerBound,
			);
			return {
				ms: performance.now() - t0,
				ops: (rows as unknown[]).length,
				rows: (rows as unknown[]).length,
				bytes: seeded.totalBytes,
			};
		},

		chatLogCount: async (c, totalBytes: number) => {
			const seeded = await seedChatLog(c.db, totalBytes);
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT COUNT(*) AS count FROM chat_log WHERE thread_id = ?",
				seeded.threadId,
			);
			return {
				ms: performance.now() - t0,
				ops: 1,
				count: (rows[0] as { count: number }).count,
				bytes: seeded.totalBytes,
			};
		},

		chatLogSum: async (c, totalBytes: number) => {
			const seeded = await seedChatLog(c.db, totalBytes);
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT SUM(content_bytes) AS total_bytes FROM chat_log WHERE thread_id = ?",
				seeded.threadId,
			);
			return {
				ms: performance.now() - t0,
				ops: 1,
				totalBytes: (rows[0] as { total_bytes: number | null }).total_bytes ?? 0,
				bytes: seeded.totalBytes,
			};
		},

		largeTxInsert500KB: async (c) => {
			const targetBytes = 500 * 1024;
			const rowSize = 4 * 1024;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		largeTxInsert1MB: async (c) => {
			const targetBytes = 1024 * 1024;
			const rowSize = 4 * 1024;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		// 1 MiB total, 4096 × 256 B rows. Max NAPI crossings.
		largeTxInsert1MBTinyRows: async (c) => {
			const targetBytes = 1024 * 1024;
			const rowSize = 256;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		// 1 MiB total, 256 × 4 KiB rows. Same shape as largeTxInsert1MB; kept as a sanity duplicate.
		largeTxInsert1MBMediumRows: async (c) => {
			const targetBytes = 1024 * 1024;
			const rowSize = 4 * 1024;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		// 1 MiB total, 1 × 1 MiB row. One NAPI crossing, exercises SQLite overflow-page chain.
		largeTxInsert1MBOneRow: async (c) => {
			const rowSize = 1024 * 1024;
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			await c.db.execute(
				"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
				rowSize,
			);
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: 1, bytes: rowSize };
		},

		largeTxInsert5MB: async (c) => {
			const targetBytes = 5 * 1024 * 1024;
			const rowSize = 4 * 1024;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		largeTxInsert10MB: async (c) => {
			const targetBytes = 10 * 1024 * 1024;
			const rowSize = 4 * 1024;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		largeTxInsert50MB: async (c) => {
			const targetBytes = 50 * 1024 * 1024;
			const rowSize = 4 * 1024;
			const rowCount = Math.ceil(targetBytes / rowSize);
			await c.db.execute(`CREATE TABLE IF NOT EXISTS large_tx (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO large_tx (payload) VALUES (randomblob(?))",
					rowSize,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: rowCount, bytes: rowCount * rowSize };
		},

		// Stress test: insert 1000 rows, delete them all, repeat 10 times.
		// Tests freelist reuse and space reclamation patterns.
		churnInsertDelete: async (c) => {
			await c.db.execute(`CREATE TABLE IF NOT EXISTS churn (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			const t0 = performance.now();
			const cycles = 10;
			const perCycle = 1000;
			for (let cycle = 0; cycle < cycles; cycle++) {
				await c.db.execute("BEGIN");
				for (let i = 0; i < perCycle; i++) {
					await c.db.execute(
						"INSERT INTO churn (payload) VALUES (randomblob(1024))",
					);
				}
				await c.db.execute("DELETE FROM churn");
				await c.db.execute("COMMIT");
			}
			return {
				ms: performance.now() - t0,
				ops: cycles * perCycle,
				cycles,
			};
		},

		// Interleave inserts, updates, deletes in same transaction. Tests how
		// the VFS handles mixed page dirtying patterns.
		mixedOltpLarge: async (c) => {
			await c.db.execute(`CREATE TABLE IF NOT EXISTS mixed_oltp (
				id INTEGER PRIMARY KEY,
				value INTEGER NOT NULL,
				data BLOB NOT NULL
			)`);
			await c.db.execute("DELETE FROM mixed_oltp");
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO mixed_oltp (id, value, data) VALUES (?, ?, randomblob(1024))",
					i,
					i * 2,
				);
			}
			await c.db.execute("COMMIT");

			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO mixed_oltp (id, value, data) VALUES (?, ?, randomblob(1024))",
					500 + i,
					i * 3,
				);
				await c.db.execute(
					"UPDATE mixed_oltp SET value = value + 1 WHERE id = ?",
					i,
				);
				if (i % 5 === 0) {
					await c.db.execute(
						"DELETE FROM mixed_oltp WHERE id = ?",
						i - 50 >= 0 ? i - 50 : i,
					);
				}
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: 500 * 2 + 100 };
		},

		// Growing aggregation: insert then SELECT SUM after each batch.
		// Tests cache invalidation and read-after-write patterns.
		growingAggregation: async (c) => {
			await c.db.execute(`CREATE TABLE IF NOT EXISTS agg_test (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value INTEGER NOT NULL
			)`);
			await c.db.execute("DELETE FROM agg_test");
			const t0 = performance.now();
			const batches = 20;
			const perBatch = 100;
			let lastSum = 0;
			for (let batch = 0; batch < batches; batch++) {
				await c.db.execute("BEGIN");
				for (let i = 0; i < perBatch; i++) {
					await c.db.execute(
						"INSERT INTO agg_test (value) VALUES (?)",
						batch * perBatch + i,
					);
				}
				await c.db.execute("COMMIT");
				const rows = (await c.db.execute(
					"SELECT SUM(value) AS s FROM agg_test",
				)) as Array<{ s: number }>;
				lastSum = rows[0]?.s ?? 0;
			}
			return {
				ms: performance.now() - t0,
				ops: batches * perBatch,
				batches,
				lastSum,
			};
		},

		// Create index on already-populated table. Tests large rewrite patterns.
		indexCreationOnLargeTable: async (c) => {
			await c.db.execute(`CREATE TABLE IF NOT EXISTS idx_test (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				key TEXT NOT NULL,
				value INTEGER NOT NULL
			)`);
			await c.db.execute("DROP INDEX IF EXISTS idx_test_key");
			await c.db.execute("DELETE FROM idx_test");
			await c.db.execute("BEGIN");
			for (let i = 0; i < 10000; i++) {
				await c.db.execute(
					"INSERT INTO idx_test (key, value) VALUES (?, ?)",
					`key-${i % 1000}-${i}`,
					i,
				);
			}
			await c.db.execute("COMMIT");
			const t0 = performance.now();
			await c.db.execute("CREATE INDEX idx_test_key ON idx_test(key)");
			return { ms: performance.now() - t0, ops: 10000 };
		},

		// Update 1000 different rows in separate UPDATEs in one transaction.
		// Stresses B-tree navigation and page dirtying.
		bulkUpdate1000Rows: async (c) => {
			await c.db.execute(`CREATE TABLE IF NOT EXISTS bulk_update (
				id INTEGER PRIMARY KEY,
				value INTEGER NOT NULL
			)`);
			await c.db.execute("DELETE FROM bulk_update");
			await c.db.execute("BEGIN");
			for (let i = 0; i < 1000; i++) {
				await c.db.execute(
					"INSERT INTO bulk_update (id, value) VALUES (?, ?)",
					i,
					i,
				);
			}
			await c.db.execute("COMMIT");

			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 1000; i++) {
				await c.db.execute(
					"UPDATE bulk_update SET value = value + 1 WHERE id = ?",
					i,
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: 1000 };
		},

		// Delete everything then re-insert. Tests truncate+regrow cycle.
		truncateAndRegrow: async (c) => {
			await c.db.execute(`CREATE TABLE IF NOT EXISTS regrow (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				payload BLOB NOT NULL
			)`);
			// Seed
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO regrow (payload) VALUES (randomblob(1024))",
				);
			}
			await c.db.execute("COMMIT");

			const t0 = performance.now();
			await c.db.execute("DELETE FROM regrow");
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO regrow (payload) VALUES (randomblob(1024))",
				);
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: 500 };
		},

		// Many small tables vs one large. Tests schema page growth.
		manySmallTables: async (c) => {
			const t0 = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 50; i++) {
				await c.db.execute(
					`CREATE TABLE IF NOT EXISTS small_t_${i} (id INTEGER PRIMARY KEY, value INTEGER)`,
				);
				for (let j = 0; j < 10; j++) {
					await c.db.execute(
						`INSERT INTO small_t_${i} (id, value) VALUES (?, ?)`,
						j,
						i * j,
					);
				}
			}
			await c.db.execute("COMMIT");
			return { ms: performance.now() - t0, ops: 50 * 10, tables: 50 };
		},
	},
});
