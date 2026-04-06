import { actor } from "rivetkit";
import { db } from "rivetkit/db";

// Generates a string of the given byte size.
function payload(bytes: number): string {
	return "x".repeat(bytes);
}

export const sqliteBench = actor({
	options: {
		actionTimeout: 300_000, // 5 minutes for large-scale benchmarks
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS bench (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					key TEXT,
					value TEXT,
					num REAL,
					created_at INTEGER NOT NULL
				)
			`);
			await db.execute(
				`CREATE INDEX IF NOT EXISTS idx_bench_key ON bench(key)`,
			);
			await db.execute(
				`CREATE INDEX IF NOT EXISTS idx_bench_num ON bench(num)`,
			);
		},
	}),
	actions: {
		// ── Migrations ──────────────────────────────────────────────

		// Create N tables each with an index to stress migration overhead.
		benchMigration: async (c, tableCount: number) => {
			const start = performance.now();
			for (let i = 0; i < tableCount; i++) {
				await c.db.execute(`
					CREATE TABLE IF NOT EXISTS migration_t${i} (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						a TEXT, b TEXT, c REAL, d INTEGER,
						created_at INTEGER NOT NULL
					)
				`);
				await c.db.execute(
					`CREATE INDEX IF NOT EXISTS idx_migration_t${i}_a ON migration_t${i}(a)`,
				);
			}
			return { tableCount, elapsedMs: performance.now() - start };
		},

		// Same as benchMigration but wrapped in a single transaction.
		benchMigrationTransaction: async (c, tableCount: number) => {
			const start = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < tableCount; i++) {
				await c.db.execute(`
					CREATE TABLE IF NOT EXISTS migration_tx_t${i} (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						a TEXT, b TEXT, c REAL, d INTEGER,
						created_at INTEGER NOT NULL
					)
				`);
				await c.db.execute(
					`CREATE INDEX IF NOT EXISTS idx_migration_tx_t${i}_a ON migration_tx_t${i}(a)`,
				);
			}
			await c.db.execute("COMMIT");
			return { tableCount, elapsedMs: performance.now() - start };
		},

		// ── Single-row inserts ──────────────────────────────────────

		benchInsertSingle: async (c, rowCount: number) => {
			const start = performance.now();
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`key-${i}`,
					`value-${i}`,
					Math.random() * 1000,
					Date.now(),
				);
			}
			return {
				rowCount,
				elapsedMs: performance.now() - start,
			};
		},

		// ── Batch inserts (multi-row VALUES) ────────────────────────

		benchInsertBatch: async (c, rowCount: number, batchSize: number = 50) => {
			const start = performance.now();
			for (let offset = 0; offset < rowCount; offset += batchSize) {
				const count = Math.min(batchSize, rowCount - offset);
				const placeholders = Array.from(
					{ length: count },
					() => "(?, ?, ?, ?)",
				).join(", ");
				const params: (string | number)[] = [];
				for (let i = 0; i < count; i++) {
					const idx = offset + i;
					params.push(
						`key-${idx}`,
						`value-${idx}`,
						Math.random() * 1000,
						Date.now(),
					);
				}
				await c.db.execute(
					`INSERT INTO bench (key, value, num, created_at) VALUES ${placeholders}`,
					...params,
				);
			}
			return {
				rowCount,
				batchSize,
				elapsedMs: performance.now() - start,
			};
		},

		// ── Transactional inserts ───────────────────────────────────

		benchInsertTransaction: async (c, rowCount: number) => {
			const start = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`key-${i}`,
					`value-${i}`,
					Math.random() * 1000,
					Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			return {
				rowCount,
				elapsedMs: performance.now() - start,
			};
		},

		// ── Point reads ─────────────────────────────────────────────

		benchPointRead: async (c, queryCount: number) => {
			// Seed data if table is empty.
			const [{ cnt }] = (await c.db.execute(
				"SELECT COUNT(*) as cnt FROM bench",
			)) as { cnt: number }[];
			if (cnt === 0) {
				await c.db.execute("BEGIN");
				for (let i = 0; i < 1000; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`key-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
				}
				await c.db.execute("COMMIT");
			}

			const start = performance.now();
			for (let i = 0; i < queryCount; i++) {
				const id = (i % 1000) + 1;
				await c.db.execute("SELECT * FROM bench WHERE id = ?", id);
			}
			return {
				queryCount,
				elapsedMs: performance.now() - start,
			};
		},

		// ── Full table scan ─────────────────────────────────────────

		benchFullScan: async (c, seedRows: number) => {
			const [{ cnt }] = (await c.db.execute(
				"SELECT COUNT(*) as cnt FROM bench",
			)) as { cnt: number }[];
			if (cnt < seedRows) {
				await c.db.execute("BEGIN");
				for (let i = cnt; i < seedRows; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`key-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
				}
				await c.db.execute("COMMIT");
			}

			const start = performance.now();
			const rows = await c.db.execute("SELECT * FROM bench");
			const elapsed = performance.now() - start;
			return {
				rowsReturned: (rows as unknown[]).length,
				elapsedMs: elapsed,
			};
		},

		// ── Range scan (indexed vs non-indexed) ─────────────────────

		benchRangeScan: async (c, seedRows: number) => {
			const [{ cnt }] = (await c.db.execute(
				"SELECT COUNT(*) as cnt FROM bench",
			)) as { cnt: number }[];
			if (cnt < seedRows) {
				await c.db.execute("BEGIN");
				for (let i = cnt; i < seedRows; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`key-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
				}
				await c.db.execute("COMMIT");
			}

			// Indexed range scan on num.
			const startIndexed = performance.now();
			const indexedRows = await c.db.execute(
				"SELECT * FROM bench WHERE num BETWEEN ? AND ?",
				0,
				seedRows / 10,
			);
			const indexedMs = performance.now() - startIndexed;

			// Non-indexed scan on value (LIKE prefix).
			const startUnindexed = performance.now();
			const unindexedRows = await c.db.execute(
				"SELECT * FROM bench WHERE value LIKE ?",
				"value-1%",
			);
			const unindexedMs = performance.now() - startUnindexed;

			return {
				seedRows,
				indexed: {
					rowsReturned: (indexedRows as unknown[]).length,
					elapsedMs: indexedMs,
				},
				unindexed: {
					rowsReturned: (unindexedRows as unknown[]).length,
					elapsedMs: unindexedMs,
				},
			};
		},

		// ── Large payloads (chunk boundary stress) ──────────────────

		benchLargePayload: async (
			c,
			rowCount: number,
			payloadBytes: number,
		) => {
			const data = payload(payloadBytes);
			const start = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`large-${i}`,
					data,
					i,
					Date.now(),
				);
			}
			await c.db.execute("COMMIT");
			const insertMs = performance.now() - start;

			// Read them back.
			const readStart = performance.now();
			const rows = await c.db.execute(
				"SELECT * FROM bench WHERE key LIKE 'large-%'",
			);
			const readMs = performance.now() - readStart;

			return {
				rowCount,
				payloadBytes,
				insertElapsedMs: insertMs,
				readElapsedMs: readMs,
				rowsRead: (rows as unknown[]).length,
			};
		},

		// ── Complex queries (JOINs, aggregations, CTEs, window fns) ─

		benchComplexQueries: async (c, seedRows: number) => {
			// Ensure we have two tables to join.
			await c.db.execute(`
				CREATE TABLE IF NOT EXISTS bench_tags (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					bench_id INTEGER NOT NULL,
					tag TEXT NOT NULL
				)
			`);
			await c.db.execute(
				`CREATE INDEX IF NOT EXISTS idx_bench_tags_bid ON bench_tags(bench_id)`,
			);

			const [{ cnt }] = (await c.db.execute(
				"SELECT COUNT(*) as cnt FROM bench",
			)) as { cnt: number }[];
			if (cnt < seedRows) {
				await c.db.execute("BEGIN");
				for (let i = cnt; i < seedRows; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`key-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
					// Add 2 tags per row.
					await c.db.execute(
						"INSERT INTO bench_tags (bench_id, tag) VALUES (?, ?), (?, ?)",
						i + 1,
						`tag-${i % 10}`,
						i + 1,
						`tag-${(i + 5) % 10}`,
					);
				}
				await c.db.execute("COMMIT");
			}

			const results: Record<string, { elapsedMs: number; rowCount: number }> =
				{};

			// JOIN
			{
				const s = performance.now();
				const rows = await c.db.execute(`
					SELECT b.id, b.key, t.tag
					FROM bench b
					JOIN bench_tags t ON t.bench_id = b.id
					WHERE b.num < 100
				`);
				results.join = {
					elapsedMs: performance.now() - s,
					rowCount: (rows as unknown[]).length,
				};
			}

			// Aggregation
			{
				const s = performance.now();
				const rows = await c.db.execute(`
					SELECT t.tag, COUNT(*) as cnt, AVG(b.num) as avg_num
					FROM bench b
					JOIN bench_tags t ON t.bench_id = b.id
					GROUP BY t.tag
					HAVING cnt > 1
					ORDER BY cnt DESC
				`);
				results.aggregation = {
					elapsedMs: performance.now() - s,
					rowCount: (rows as unknown[]).length,
				};
			}

			// CTE
			{
				const s = performance.now();
				const rows = await c.db.execute(`
					WITH ranked AS (
						SELECT id, key, num,
							ROW_NUMBER() OVER (ORDER BY num DESC) as rank
						FROM bench
					)
					SELECT * FROM ranked WHERE rank <= 50
				`);
				results.cte_window = {
					elapsedMs: performance.now() - s,
					rowCount: (rows as unknown[]).length,
				};
			}

			// Subquery
			{
				const s = performance.now();
				const rows = await c.db.execute(`
					SELECT * FROM bench
					WHERE id IN (
						SELECT bench_id FROM bench_tags WHERE tag = 'tag-0'
					)
				`);
				results.subquery = {
					elapsedMs: performance.now() - s,
					rowCount: (rows as unknown[]).length,
				};
			}

			return { seedRows, results };
		},

		// ── Bulk update ─────────────────────────────────────────────

		benchBulkUpdate: async (c, seedRows: number) => {
			const [{ cnt }] = (await c.db.execute(
				"SELECT COUNT(*) as cnt FROM bench",
			)) as { cnt: number }[];
			if (cnt < seedRows) {
				await c.db.execute("BEGIN");
				for (let i = cnt; i < seedRows; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`key-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
				}
				await c.db.execute("COMMIT");
			}

			const start = performance.now();
			await c.db.execute(
				"UPDATE bench SET value = 'updated', num = num + 1 WHERE num < ?",
				seedRows / 2,
			);
			return {
				seedRows,
				elapsedMs: performance.now() - start,
			};
		},

		// ── Bulk delete + VACUUM ────────────────────────────────────

		benchDeleteVacuum: async (c, seedRows: number) => {
			const [{ cnt }] = (await c.db.execute(
				"SELECT COUNT(*) as cnt FROM bench",
			)) as { cnt: number }[];
			if (cnt < seedRows) {
				await c.db.execute("BEGIN");
				for (let i = cnt; i < seedRows; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`key-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
				}
				await c.db.execute("COMMIT");
			}

			// Delete half the rows.
			const deleteStart = performance.now();
			await c.db.execute("DELETE FROM bench WHERE num < ?", seedRows / 2);
			const deleteMs = performance.now() - deleteStart;

			// VACUUM.
			const vacuumStart = performance.now();
			await c.db.execute("VACUUM");
			const vacuumMs = performance.now() - vacuumStart;

			const [{ remaining }] = (await c.db.execute(
				"SELECT COUNT(*) as remaining FROM bench",
			)) as { remaining: number }[];

			return {
				seedRows,
				deleteElapsedMs: deleteMs,
				vacuumElapsedMs: vacuumMs,
				remainingRows: remaining,
			};
		},

		// ── Mixed OLTP (interleaved reads + writes) ─────────────────

		benchMixedOltp: async (
			c,
			operationCount: number,
			readRatio: number = 0.7,
		) => {
			// Seed some data.
			await c.db.execute("BEGIN");
			for (let i = 0; i < 500; i++) {
				await c.db.execute(
					"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
					`oltp-${i}`,
					`value-${i}`,
					i,
					Date.now(),
				);
			}
			await c.db.execute("COMMIT");

			let reads = 0;
			let writes = 0;
			const start = performance.now();

			for (let i = 0; i < operationCount; i++) {
				if (Math.random() < readRatio) {
					const id = Math.floor(Math.random() * 500) + 1;
					await c.db.execute("SELECT * FROM bench WHERE id = ?", id);
					reads++;
				} else {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`oltp-new-${i}`,
						`value-${i}`,
						i,
						Date.now(),
					);
					writes++;
				}
			}
			return {
				operationCount,
				reads,
				writes,
				elapsedMs: performance.now() - start,
			};
		},

		// ── Hot row (write amplification) ───────────────────────────

		benchHotRow: async (c, updateCount: number) => {
			await c.db.execute(
				"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
				"hot-row",
				"initial",
				0,
				Date.now(),
			);
			const [{ id: hotId }] = (await c.db.execute(
				"SELECT id FROM bench WHERE key = 'hot-row' LIMIT 1",
			)) as { id: number }[];

			const start = performance.now();
			for (let i = 0; i < updateCount; i++) {
				await c.db.execute(
					"UPDATE bench SET value = ?, num = ? WHERE id = ?",
					`updated-${i}`,
					i,
					hotId,
				);
			}
			return {
				updateCount,
				elapsedMs: performance.now() - start,
			};
		},

		// ── JSON operations ─────────────────────────────────────────

		benchJson: async (c, rowCount: number) => {
			await c.db.execute(`
				CREATE TABLE IF NOT EXISTS bench_json (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					data TEXT NOT NULL
				)
			`);

			// Insert JSON rows.
			const insertStart = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < rowCount; i++) {
				const json = JSON.stringify({
					name: `user-${i}`,
					age: 20 + (i % 50),
					tags: [`tag-${i % 5}`, `tag-${(i + 1) % 5}`],
					address: {
						city: `city-${i % 20}`,
						zip: `${10000 + i}`,
					},
				});
				await c.db.execute(
					"INSERT INTO bench_json (data) VALUES (?)",
					json,
				);
			}
			await c.db.execute("COMMIT");
			const insertMs = performance.now() - insertStart;

			// json_extract query.
			const extractStart = performance.now();
			const extractRows = await c.db.execute(`
				SELECT id, json_extract(data, '$.name') as name,
					json_extract(data, '$.age') as age,
					json_extract(data, '$.address.city') as city
				FROM bench_json
				WHERE json_extract(data, '$.age') > 40
			`);
			const extractMs = performance.now() - extractStart;

			// json_each aggregation.
			const eachStart = performance.now();
			const eachRows = await c.db.execute(`
				SELECT value as tag, COUNT(*) as cnt
				FROM bench_json, json_each(json_extract(data, '$.tags'))
				GROUP BY value
				ORDER BY cnt DESC
			`);
			const eachMs = performance.now() - eachStart;

			return {
				rowCount,
				insertElapsedMs: insertMs,
				jsonExtract: {
					elapsedMs: extractMs,
					rowCount: (extractRows as unknown[]).length,
				},
				jsonEach: {
					elapsedMs: eachMs,
					rowCount: (eachRows as unknown[]).length,
				},
			};
		},

		// ── FTS5 full-text search ───────────────────────────────────

		benchFts: async (c, docCount: number) => {
			await c.db.execute(`
				CREATE VIRTUAL TABLE IF NOT EXISTS bench_fts
				USING fts5(title, body)
			`);

			const words = [
				"alpha", "bravo", "charlie", "delta", "echo",
				"foxtrot", "golf", "hotel", "india", "juliet",
				"kilo", "lima", "mike", "november", "oscar",
			];
			function randomSentence(len: number): string {
				return Array.from(
					{ length: len },
					() => words[Math.floor(Math.random() * words.length)],
				).join(" ");
			}

			// Insert documents.
			const insertStart = performance.now();
			await c.db.execute("BEGIN");
			for (let i = 0; i < docCount; i++) {
				await c.db.execute(
					"INSERT INTO bench_fts (title, body) VALUES (?, ?)",
					randomSentence(5),
					randomSentence(50),
				);
			}
			await c.db.execute("COMMIT");
			const insertMs = performance.now() - insertStart;

			// Search.
			const searchStart = performance.now();
			const searchRows = await c.db.execute(`
				SELECT * FROM bench_fts WHERE bench_fts MATCH 'alpha AND bravo'
				ORDER BY rank
				LIMIT 50
			`);
			const searchMs = performance.now() - searchStart;

			// Prefix search.
			const prefixStart = performance.now();
			const prefixRows = await c.db.execute(`
				SELECT * FROM bench_fts WHERE bench_fts MATCH 'cha*'
				ORDER BY rank
				LIMIT 50
			`);
			const prefixMs = performance.now() - prefixStart;

			return {
				docCount,
				insertElapsedMs: insertMs,
				search: {
					elapsedMs: searchMs,
					rowCount: (searchRows as unknown[]).length,
				},
				prefixSearch: {
					elapsedMs: prefixMs,
					rowCount: (prefixRows as unknown[]).length,
				},
			};
		},

		// ── Database growth (throughput at different sizes) ──────────

		benchGrowth: async (c, targetRows: number, measureInterval: number) => {
			const measurements: {
				rowCount: number;
				insertBatchMs: number;
				pointReadMs: number;
			}[] = [];

			let totalInserted = 0;
			while (totalInserted < targetRows) {
				const batchCount = Math.min(measureInterval, targetRows - totalInserted);

				// Measure insert batch.
				const insertStart = performance.now();
				await c.db.execute("BEGIN");
				for (let i = 0; i < batchCount; i++) {
					await c.db.execute(
						"INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)",
						`grow-${totalInserted + i}`,
						`value-${totalInserted + i}`,
						totalInserted + i,
						Date.now(),
					);
				}
				await c.db.execute("COMMIT");
				const insertMs = performance.now() - insertStart;
				totalInserted += batchCount;

				// Measure point read at current size.
				const readStart = performance.now();
				for (let i = 0; i < 100; i++) {
					const id = Math.floor(Math.random() * totalInserted) + 1;
					await c.db.execute("SELECT * FROM bench WHERE id = ?", id);
				}
				const readMs = performance.now() - readStart;

				measurements.push({
					rowCount: totalInserted,
					insertBatchMs: insertMs,
					pointReadMs: readMs,
				});
			}

			return { targetRows, measureInterval, measurements };
		},

		// ── Utility: reset tables ───────────────────────────────────

		reset: async (c) => {
			await c.db.execute("DELETE FROM bench");
			await c.db.execute(
				"DELETE FROM sqlite_sequence WHERE name='bench'",
			);
			return { ok: true };
		},
	},
});
