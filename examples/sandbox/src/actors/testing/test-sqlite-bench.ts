import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const testSqliteBench = actor({
	options: {
		actionTimeout: 300_000,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`CREATE TABLE IF NOT EXISTS bench (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				key TEXT NOT NULL,
				value TEXT NOT NULL,
				num INTEGER NOT NULL DEFAULT 0,
				payload BLOB,
				created_at INTEGER NOT NULL DEFAULT 0
			)`);
			await db.execute("CREATE INDEX IF NOT EXISTS idx_bench_key ON bench(key)");
			await db.execute("CREATE INDEX IF NOT EXISTS idx_bench_num ON bench(num)");

			await db.execute(`CREATE TABLE IF NOT EXISTS bench_json (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				data TEXT NOT NULL DEFAULT '{}'
			)`);

			await db.execute(`CREATE TABLE IF NOT EXISTS bench_secondary (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				bench_id INTEGER NOT NULL,
				label TEXT NOT NULL,
				score REAL NOT NULL DEFAULT 0,
				FOREIGN KEY (bench_id) REFERENCES bench(id)
			)`);
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
			const updated = await c.db.execute("SELECT * FROM bench WHERE id = ?", id);
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
				"SELECT id, json_extract(data, '$.name') as name, json_extract(data, '$.score') as score FROM bench_json ORDER BY json_extract(data, '$.score') DESC LIMIT 10"
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
				"SELECT SUM(json_extract(value, '$.val')) as total FROM bench_json, json_each(json_extract(data, '$.items')) LIMIT 1"
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
				"SELECT key, COUNT(*) as cnt, AVG(num) as avg_num, MIN(num) as min_num, MAX(num) as max_num FROM bench WHERE key LIKE 'grp-%' GROUP BY key ORDER BY cnt DESC"
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
				"SELECT * FROM bench WHERE num > (SELECT AVG(num) FROM bench) ORDER BY num DESC LIMIT 50"
			);
			return { ms: performance.now() - t0, seedMs, rows: (rows as unknown[]).length };
		},

		complexJoin: async (c) => {
			const t0Seed = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < 200; i++) {
				await c.db.execute("INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)", `j-${i}`, `v-${i}`, i, Date.now());
				await c.db.execute("INSERT INTO bench_secondary (bench_id, label, score) VALUES (?, ?, ?)", i + 1, `label-${i}`, Math.random() * 100);
			}
			await c.db.execute("COMMIT");
			const seedMs = performance.now() - t0Seed;
			const t0 = performance.now();
			const rows = await c.db.execute(
				"SELECT b.key, b.num, s.label, s.score FROM bench b INNER JOIN bench_secondary s ON s.bench_id = b.id WHERE b.key LIKE 'j-%' ORDER BY s.score DESC LIMIT 200"
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
	},
});
