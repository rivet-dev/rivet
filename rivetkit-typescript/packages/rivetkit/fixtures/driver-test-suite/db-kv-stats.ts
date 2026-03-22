import { actor } from "rivetkit";
import type { DatabaseProvider, DatabaseProviderContext, RawAccess } from "rivetkit/db";
import { AsyncMutex, toSqliteBindings } from "../../src/db/shared";
import type { KvVfsOptions } from "@rivetkit/sqlite-vfs";

export interface KvStats {
	getBatchCalls: number;
	getBatchKeys: number;
	putBatchCalls: number;
	putBatchEntries: number;
	deleteBatchCalls: number;
}

export interface KvLogEntry {
	op: string;
	keys: string[];
}

const FILE_TAGS: Record<number, string> = { 0: "main", 1: "journal", 2: "wal", 3: "shm" };

function decodeKey(key: Uint8Array): string {
	if (key.length < 4 || key[0] !== 8 || key[1] !== 1) {
		return `unknown(${Array.from(key).join(",")})`;
	}
	const prefix = key[2]; // 0 = meta, 1 = chunk
	const fileTag = FILE_TAGS[key[3]] ?? `file${key[3]}`;
	if (prefix === 0) {
		return `meta:${fileTag}`;
	}
	if (prefix === 1 && key.length === 8) {
		const chunkIndex = (key[4] << 24) | (key[5] << 16) | (key[6] << 8) | key[7];
		return `chunk:${fileTag}[${chunkIndex}]`;
	}
	return `unknown(${Array.from(key).join(",")})`;
}

function instrumentedKvStore(
	kv: DatabaseProviderContext["kv"],
	stats: KvStats,
	log: KvLogEntry[],
): KvVfsOptions {
	return {
		get: async (key: Uint8Array) => {
			stats.getBatchCalls++;
			stats.getBatchKeys++;
			log.push({ op: "get", keys: [decodeKey(key)] });
			const results = await kv.batchGet([key]);
			return results[0] ?? null;
		},
		getBatch: async (keys: Uint8Array[]) => {
			stats.getBatchCalls++;
			stats.getBatchKeys += keys.length;
			log.push({ op: "getBatch", keys: keys.map(decodeKey) });
			return await kv.batchGet(keys);
		},
		put: async (key: Uint8Array, value: Uint8Array) => {
			stats.putBatchCalls++;
			stats.putBatchEntries++;
			log.push({ op: "put", keys: [decodeKey(key)] });
			await kv.batchPut([[key, value]]);
		},
		putBatch: async (entries: [Uint8Array, Uint8Array][]) => {
			stats.putBatchCalls++;
			stats.putBatchEntries += entries.length;
			log.push({ op: "putBatch", keys: entries.map(([k]) => decodeKey(k)) });
			await kv.batchPut(entries);
		},
		deleteBatch: async (keys: Uint8Array[]) => {
			stats.deleteBatchCalls++;
			log.push({ op: "deleteBatch", keys: keys.map(decodeKey) });
			await kv.batchDelete(keys);
		},
	};
}

interface ActorKvData {
	stats: KvStats;
	log: KvLogEntry[];
}

const perActorData = new Map<string, ActorKvData>();

function getOrCreateData(actorId: string): ActorKvData {
	let d = perActorData.get(actorId);
	if (!d) {
		d = {
			stats: { getBatchCalls: 0, getBatchKeys: 0, putBatchCalls: 0, putBatchEntries: 0, deleteBatchCalls: 0 },
			log: [],
		};
		perActorData.set(actorId, d);
	}
	return d;
}

const provider: DatabaseProvider<RawAccess> = {
	createClient: async (ctx) => {
		if (!ctx.sqliteVfs) {
			throw new Error("SqliteVfs instance not provided in context.");
		}

		const data = getOrCreateData(ctx.actorId);
		const kvStore = instrumentedKvStore(ctx.kv, data.stats, data.log);
		const db = await ctx.sqliteVfs.open(ctx.actorId, kvStore);
		let closed = false;
		const mutex = new AsyncMutex();
		const ensureOpen = () => {
			if (closed) throw new Error("database is closed");
		};

		return {
			execute: async <
				TRow extends Record<string, unknown> = Record<string, unknown>,
			>(
				query: string,
				...args: unknown[]
			): Promise<TRow[]> => {
				return await mutex.run(async () => {
					ensureOpen();
					if (args.length > 0) {
						const bindings = toSqliteBindings(args);
						const token = query.trimStart().slice(0, 16).toUpperCase();
						const returnsRows =
							token.startsWith("SELECT") ||
							token.startsWith("PRAGMA") ||
							token.startsWith("WITH");
						if (returnsRows) {
							const { rows, columns } = await db.query(query, bindings);
							return rows.map((row: unknown[]) => {
								const rowObj: Record<string, unknown> = {};
								for (let i = 0; i < columns.length; i++) {
									rowObj[columns[i]] = row[i];
								}
								return rowObj;
							}) as TRow[];
						}
						await db.run(query, bindings);
						return [] as TRow[];
					}
					const results: Record<string, unknown>[] = [];
					let columnNames: string[] | null = null;
					await db.exec(query, (row: unknown[], columns: string[]) => {
						if (!columnNames) columnNames = columns;
						const rowObj: Record<string, unknown> = {};
						for (let i = 0; i < row.length; i++) {
							rowObj[columnNames[i]] = row[i];
						}
						results.push(rowObj);
					});
					return results as TRow[];
				});
			},
			close: async () => {
				const shouldClose = await mutex.run(async () => {
					if (closed) return false;
					closed = true;
					return true;
				});
				if (shouldClose) {
					await db.close();
				}
			},
		} satisfies RawAccess;
	},
	onMigrate: async (client) => {
		await client.execute(`
			CREATE TABLE IF NOT EXISTS counter (
				id INTEGER PRIMARY KEY,
				count INTEGER NOT NULL DEFAULT 0
			)
		`);
		await client.execute(`INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)`);
	},
	onDestroy: async (client) => {
		await client.close();
	},
};

export const dbKvStatsActor = actor({
	state: {} as Record<string, never>,
	db: provider,
	actions: {
		warmUp: async (c) => {
			// Prime migrations and pager cache. The first execute triggers
			// the migration (CREATE TABLE + INSERT), which loads pages
			// from KV into the pager cache. The second write ensures all
			// dirty pages are flushed and the cache is fully warmed.
			await c.db.execute(`UPDATE counter SET count = count + 1 WHERE id = 1`);
			await c.db.execute(`UPDATE counter SET count = count + 1 WHERE id = 1`);
			const data = getOrCreateData(c.actorId);
			data.stats.getBatchCalls = 0;
			data.stats.getBatchKeys = 0;
			data.stats.putBatchCalls = 0;
			data.stats.putBatchEntries = 0;
			data.stats.deleteBatchCalls = 0;
			data.log.length = 0;
		},

		resetStats: (c) => {
			const data = getOrCreateData(c.actorId);
			data.stats.getBatchCalls = 0;
			data.stats.getBatchKeys = 0;
			data.stats.putBatchCalls = 0;
			data.stats.putBatchEntries = 0;
			data.stats.deleteBatchCalls = 0;
			data.log.length = 0;
		},

		getStats: (c) => {
			return { ...getOrCreateData(c.actorId).stats };
		},

		getLog: (c) => {
			return getOrCreateData(c.actorId).log;
		},

		increment: async (c) => {
			await c.db.execute(`UPDATE counter SET count = count + 1 WHERE id = 1`);
		},

		getCount: async (c) => {
			const rows = await c.db.execute<{ count: number }>(
				`SELECT count FROM counter WHERE id = 1`,
			);
			return rows[0]?.count ?? 0;
		},

		incrementAndRead: async (c) => {
			await c.db.execute(`UPDATE counter SET count = count + 1 WHERE id = 1`);
			const rows = await c.db.execute<{ count: number }>(
				`SELECT count FROM counter WHERE id = 1`,
			);
			return rows[0]?.count ?? 0;
		},

		insertWithIndex: async (c) => {
			await c.db.execute(`
				CREATE TABLE IF NOT EXISTS indexed_data (
					id INTEGER PRIMARY KEY,
					value TEXT NOT NULL
				)
			`);
			await c.db.execute(`
				CREATE INDEX IF NOT EXISTS idx_indexed_data_value ON indexed_data(value)
			`);
			await c.db.execute(
				`INSERT INTO indexed_data (value) VALUES (?)`,
				`row-${Date.now()}`,
			);
		},

		rollbackTest: async (c) => {
			await c.db.execute(`
				CREATE TABLE IF NOT EXISTS rollback_test (
					id INTEGER PRIMARY KEY,
					value TEXT NOT NULL
				)
			`);
			await c.db.execute(`
				BEGIN;
				INSERT INTO rollback_test (value) VALUES ('should-not-persist');
				ROLLBACK;
			`);
		},

		multiStmtTx: async (c) => {
			await c.db.execute(`
				CREATE TABLE IF NOT EXISTS multi_stmt (
					id INTEGER PRIMARY KEY,
					value TEXT NOT NULL
				)
			`);
			await c.db.execute(`
				BEGIN;
				INSERT INTO multi_stmt (value) VALUES ('row-a');
				INSERT INTO multi_stmt (value) VALUES ('row-b');
				COMMIT;
			`);
		},

		bulkInsertLarge: async (c) => {
			await c.db.execute(`
				CREATE TABLE IF NOT EXISTS bulk_data (
					id INTEGER PRIMARY KEY,
					payload TEXT NOT NULL
				)
			`);
			const pad = "x".repeat(4000);
			const stmts = ["BEGIN;"];
			for (let i = 0; i < 200; i++) {
				const escaped = `bulk-${i}-${pad}`.replace(/'/g, "''");
				stmts.push(`INSERT INTO bulk_data (payload) VALUES ('${escaped}');`);
			}
			stmts.push("COMMIT;");
			await c.db.execute(stmts.join("\n"));
		},

		getRowCount: async (c) => {
			const rows = await c.db.execute<{ cnt: number }>(
				`SELECT COUNT(*) as cnt FROM bulk_data`,
			);
			return rows[0]?.cnt ?? 0;
		},

		runIntegrityCheck: async (c) => {
			const rows = await c.db.execute<{ integrity_check: string }>(
				`PRAGMA integrity_check`,
			);
			return rows[0]?.integrity_check ?? "unknown";
		},

		triggerSleep: (c) => {
			c.sleep();
		},
	},
	options: {
		sleepTimeout: 100,
	},
});
