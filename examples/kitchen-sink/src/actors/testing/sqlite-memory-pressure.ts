import { actor } from "rivetkit";
import type { SqliteNativeMetrics } from "rivetkit/db";
import { db } from "rivetkit/db";

interface RunCycleInput {
	seed: string;
	cycle: number;
	insertRows?: number;
	rowBytes?: number;
	deleteRows?: number;
	retainRows?: number;
	scanRows?: number;
}

interface CountRow {
	count: number;
}

interface StorageRow {
	page_count: number;
	freelist_count: number;
	page_size: number;
	vfs: SqliteNativeMetrics | null;
}

const DEFAULT_INSERT_ROWS = 128;
const DEFAULT_ROW_BYTES = 16 * 1024;
// const DEFAULT_DELETE_ROWS = 64;
// const DEFAULT_RETAIN_ROWS = 1024;
const DEFAULT_SCAN_ROWS = 512;
const INSERT_BATCH_ROWS = 32;

function finiteInt(value: number | undefined, fallback: number): number {
	if (value === undefined) return fallback;
	if (!Number.isFinite(value) || value < 0) {
		throw new Error(`expected a non-negative finite number, got ${value}`);
	}
	return Math.floor(value);
}

function copyNativeMetrics(
	metrics: SqliteNativeMetrics | null | undefined,
): SqliteNativeMetrics | null {
	if (!metrics) return null;
	const raw = metrics as unknown as Record<string, unknown>;
	const numberField = (camel: string, snake: string) =>
		Number(raw[camel] ?? raw[snake] ?? 0);
	return {
		requestBuildNs: numberField("requestBuildNs", "request_build_ns"),
		serializeNs: numberField("serializeNs", "serialize_ns"),
		transportNs: numberField("transportNs", "transport_ns"),
		stateUpdateNs: numberField("stateUpdateNs", "state_update_ns"),
		totalNs: numberField("totalNs", "total_ns"),
		commitCount: numberField("commitCount", "commit_count"),
		pageCacheEntries: numberField("pageCacheEntries", "page_cache_entries"),
		pageCacheWeightedSize: numberField(
			"pageCacheWeightedSize",
			"page_cache_weighted_size",
		),
		pageCacheCapacityPages: numberField(
			"pageCacheCapacityPages",
			"page_cache_capacity_pages",
		),
		writeBufferDirtyPages: numberField(
			"writeBufferDirtyPages",
			"write_buffer_dirty_pages",
		),
		dbSizePages: numberField("dbSizePages", "db_size_pages"),
	};
}

async function queryOne<T>(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	sql: string,
	...args: unknown[]
): Promise<T> {
	const rows = await database.execute(sql, ...args);
	if (!rows[0]) throw new Error(`query returned no rows: ${sql}`);
	return rows[0] as T;
}

async function storageStats(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	nativeMetrics?: () =>
		| SqliteNativeMetrics
		| Promise<SqliteNativeMetrics | null>
		| null;
}): Promise<StorageRow> {
	const [pageCount, freelistCount, pageSize] = await Promise.all([
		queryOne<{ page_count: number }>(database, "PRAGMA page_count"),
		queryOne<{ freelist_count: number }>(database, "PRAGMA freelist_count"),
		queryOne<{ page_size: number }>(database, "PRAGMA page_size"),
	]);

	const nativeMetrics = await database.nativeMetrics?.();
	const copiedMetrics = copyNativeMetrics(nativeMetrics);

	return {
		page_count: pageCount.page_count,
		freelist_count: freelistCount.freelist_count,
		page_size: pageSize.page_size,
		vfs: copiedMetrics,
	};
}

export const sqliteMemoryPressure = actor({
	options: {
		actionTimeout: 300_000,
	},
	state: {
		sleepCount: 0,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS pressure_rows (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					seed TEXT NOT NULL,
					cycle INTEGER NOT NULL,
					bucket INTEGER NOT NULL,
					payload BLOB NOT NULL,
					touched_count INTEGER NOT NULL DEFAULT 0,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_pressure_rows_seed_cycle ON pressure_rows(seed, cycle)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_pressure_rows_bucket ON pressure_rows(bucket)",
			);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS pressure_cycles (
					cycle INTEGER PRIMARY KEY,
					seed TEXT NOT NULL,
					inserted_rows INTEGER NOT NULL,
					deleted_rows INTEGER NOT NULL,
					active_rows INTEGER NOT NULL,
					active_bytes INTEGER NOT NULL,
					duration_ms REAL NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	onSleep: (c) => {
		c.state.sleepCount += 1;
		console.log(
			JSON.stringify({
				kind: "sqlite_memory_pressure_on_sleep",
				actorId: c.actorId,
				sleepCount: c.state.sleepCount,
				timestamp: new Date().toISOString(),
			}),
		);
	},
	actions: {
		reset: async (c) => {
			await c.db.execute("DELETE FROM pressure_cycles");
			await c.db.execute("DELETE FROM pressure_rows");
			await c.db.execute("VACUUM");
			return {
				ok: true,
				storage: await storageStats(c.db),
			};
		},

		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},

		releaseStorage: async (c) => {
			const before = await storageStats(c.db);
			// Keep the remote DB large for the sleep reclamation soak.
			// await c.db.execute("DELETE FROM pressure_cycles");
			// await c.db.execute("DELETE FROM pressure_rows");
			// await c.db.execute("VACUUM");
			return {
				ok: true,
				before,
				after: await storageStats(c.db),
			};
		},

		stats: async (c) => {
			const rowStats = await queryOne<{
				active_rows: number;
				active_bytes: number | null;
				touched_sum: number | null;
			}>(
				c.db,
				"SELECT COUNT(*) AS active_rows, COALESCE(SUM(length(payload)), 0) AS active_bytes, COALESCE(SUM(touched_count), 0) AS touched_sum FROM pressure_rows",
			);
			const cycles = await queryOne<CountRow>(
				c.db,
				"SELECT COUNT(*) AS count FROM pressure_cycles",
			);
			const integrity = await queryOne<{ integrity_check: string }>(
				c.db,
				"PRAGMA integrity_check",
			);

			return {
				activeRows: rowStats.active_rows,
				activeBytes: rowStats.active_bytes ?? 0,
				touchedCount: rowStats.touched_sum ?? 0,
				cycles: cycles.count,
				integrityCheck: integrity.integrity_check,
				storage: await storageStats(c.db),
			};
		},

		runCycle: async (c, input: RunCycleInput) => {
			const startedAt = performance.now();
			const insertRows = finiteInt(input.insertRows, DEFAULT_INSERT_ROWS);
			const rowBytes = finiteInt(input.rowBytes, DEFAULT_ROW_BYTES);
			// const deleteRows = finiteInt(input.deleteRows, DEFAULT_DELETE_ROWS);
			// const retainRows = Math.max(
			// 	1,
			// 	finiteInt(input.retainRows, DEFAULT_RETAIN_ROWS),
			// );
			const scanRows = Math.max(1, finiteInt(input.scanRows, DEFAULT_SCAN_ROWS));
			const now = Date.now();
			let insertedRows = 0;
			const logStage = (
				stage: string,
				phase: "start" | "end" | "error",
				fields: Record<string, unknown> = {},
			) => {
				console.log(
					JSON.stringify({
						kind: "sqlite_memory_pressure_run_cycle_stage",
						actorId: c.actorId,
						seed: input.seed,
						cycle: input.cycle,
						stage,
						phase,
						elapsedMs: performance.now() - startedAt,
						timestamp: new Date().toISOString(),
						...fields,
					}),
				);
			};
			const executeTimed = async (
				stage: string,
				sql: string,
				...args: unknown[]
			) => {
				const stageStartedAt = performance.now();
				logStage(stage, "start", { argCount: args.length });
				try {
					const rows = await c.db.execute(sql, ...args);
					logStage(stage, "end", {
						durationMs: performance.now() - stageStartedAt,
						rowCount: rows.length,
					});
					return rows;
				} catch (err) {
					logStage(stage, "error", {
						durationMs: performance.now() - stageStartedAt,
						error: err instanceof Error ? err.message : String(err),
					});
					throw err;
				}
			};
			logStage("run_cycle", "start", {
				insertRows,
				rowBytes,
				scanRows,
			});

			await executeTimed("begin", "BEGIN");
			try {
				while (insertedRows < insertRows) {
					const batchRows = Math.min(
						INSERT_BATCH_ROWS,
						insertRows - insertedRows,
					);
					const placeholders: string[] = [];
					const args: unknown[] = [];

					for (let i = 0; i < batchRows; i += 1) {
						const rowIndex = insertedRows + i;
						placeholders.push("(?, ?, ?, randomblob(?), 0, ?)");
						args.push(
							input.seed,
							input.cycle,
							(input.cycle + rowIndex) % 32,
							rowBytes,
							now + rowIndex,
						);
					}

					await executeTimed(
						"insert_batch",
						`INSERT INTO pressure_rows (seed, cycle, bucket, payload, touched_count, created_at) VALUES ${placeholders.join(", ")}`,
						...args,
					);
					insertedRows += batchRows;
					logStage("insert_batch_progress", "end", {
						insertedRows,
						batchRows,
					});
				}
				await executeTimed("commit", "COMMIT");
			} catch (err) {
				await executeTimed("rollback", "ROLLBACK").catch(() => undefined);
				throw err;
			}

			const scan = await executeTimed(
				"scan_recent",
				"SELECT id, length(payload) AS payload_bytes FROM pressure_rows ORDER BY id DESC LIMIT ?",
				scanRows,
			);
			const bucketAgg = await executeTimed(
				"bucket_agg",
				"SELECT bucket, COUNT(*) AS rows, SUM(length(payload)) AS bytes FROM pressure_rows WHERE bucket BETWEEN ? AND ? GROUP BY bucket ORDER BY bucket",
				input.cycle % 16,
				(input.cycle % 16) + 15,
			);
			await executeTimed(
				"touch_recent",
				"UPDATE pressure_rows SET touched_count = touched_count + 1 WHERE id IN (SELECT id FROM pressure_rows ORDER BY id DESC LIMIT ?)",
				Math.min(scanRows, insertRows),
			);

			let deletedRows = 0;
			// const beforeDelete = await queryOne<CountRow>(
			// 	c.db,
			// 	"SELECT COUNT(*) AS count FROM pressure_rows",
			// );
			// const overRetainRows = Math.max(0, beforeDelete.count - retainRows);
			// const deleteLimit = Math.max(deleteRows, overRetainRows);
			// if (deleteLimit > 0) {
			// 	await c.db.execute(
			// 		"DELETE FROM pressure_rows WHERE id IN (SELECT id FROM pressure_rows ORDER BY id ASC LIMIT ?)",
			// 		deleteLimit,
			// 	);
			// 	const afterDelete = await queryOne<CountRow>(
			// 		c.db,
			// 		"SELECT changes() AS count",
			// 	);
			// 	deletedRows = afterDelete.count;
			// }

			const rowStatsRows = await executeTimed(
				"row_stats",
				"SELECT COUNT(*) AS active_rows, COALESCE(SUM(length(payload)), 0) AS active_bytes FROM pressure_rows",
			);
			const rowStats = rowStatsRows[0] as
				| {
						active_rows: number;
						active_bytes: number | null;
				  }
				| undefined;
			if (!rowStats) throw new Error("query returned no rows: row_stats");
			const integrityRows = await executeTimed(
				"integrity_check",
				"PRAGMA integrity_check",
			);
			const integrity = integrityRows[0] as
				| { integrity_check: string }
				| undefined;
			if (!integrity) {
				throw new Error("query returned no rows: integrity_check");
			}
			const durationMs = performance.now() - startedAt;

			await executeTimed(
				"record_cycle",
				"INSERT OR REPLACE INTO pressure_cycles (cycle, seed, inserted_rows, deleted_rows, active_rows, active_bytes, duration_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
				input.cycle,
				input.seed,
				insertedRows,
				deletedRows,
				rowStats.active_rows,
				rowStats.active_bytes ?? 0,
				durationMs,
				now,
			);

			const storageStartedAt = performance.now();
			logStage("storage_stats", "start");
			const storage = await storageStats(c.db);
			logStage("storage_stats", "end", {
				durationMs: performance.now() - storageStartedAt,
				pageCount: storage.page_count,
				dbSizePages: storage.vfs?.dbSizePages ?? null,
				pageCacheEntries: storage.vfs?.pageCacheEntries ?? null,
			});
			logStage("run_cycle", "end", {
				durationMs,
				activeRows: rowStats.active_rows,
				activeBytes: rowStats.active_bytes ?? 0,
				pageCount: storage.page_count,
			});

			return {
				seed: input.seed,
				cycle: input.cycle,
				insertedRows,
				deletedRows,
				activeRows: rowStats.active_rows,
				activeBytes: rowStats.active_bytes ?? 0,
				scannedRows: scan.length,
				bucketsRead: bucketAgg.length,
				integrityCheck: integrity.integrity_check,
				storage,
				durationMs,
			};
		},
	},
});
