import { describe, expect, it } from "vitest";
import { createSqliteVfs } from "../src/backend";
import type { KvVfsOptions } from "@rivetkit/sqlite-vfs";

// -- Instrumented KV store --

interface KvStats {
	getCalls: number;
	getKeys: number;
	getBatchCalls: number;
	getBatchKeys: number;
	putCalls: number;
	putKeys: number;
	putBatchCalls: number;
	putBatchKeys: number;
	deleteBatchCalls: number;
}

interface KvLogEntry {
	op: string;
	keys: string[];
}

const FILE_TAGS: Record<number, string> = {
	0: "main",
	1: "journal",
	2: "wal",
	3: "shm",
};

function decodeKey(key: Uint8Array): string {
	if (key.length < 4 || key[0] !== 8 || key[1] !== 1) {
		return `unknown(${Array.from(key).join(",")})`;
	}
	const prefix = key[2];
	const fileTag = FILE_TAGS[key[3]] ?? `file${key[3]}`;
	if (prefix === 0) return `meta:${fileTag}`;
	if (prefix === 1 && key.length === 8) {
		const chunkIndex =
			(key[4] << 24) | (key[5] << 16) | (key[6] << 8) | key[7];
		return `chunk:${fileTag}[${chunkIndex}]`;
	}
	return `unknown(${Array.from(key).join(",")})`;
}

function createInstrumentedKvStore(): {
	kvStore: KvVfsOptions;
	stats: KvStats;
	log: KvLogEntry[];
	resetStats: () => void;
} {
	const store = new Map<string, Uint8Array>();
	const log: KvLogEntry[] = [];
	const stats: KvStats = {
		getCalls: 0,
		getKeys: 0,
		getBatchCalls: 0,
		getBatchKeys: 0,
		putCalls: 0,
		putKeys: 0,
		putBatchCalls: 0,
		putBatchKeys: 0,
		deleteBatchCalls: 0,
	};

	function keyToString(key: Uint8Array): string {
		return Buffer.from(key).toString("hex");
	}

	const kvStore: KvVfsOptions = {
		get: async (key) => {
			stats.getCalls++;
			stats.getKeys++;
			log.push({ op: "get", keys: [decodeKey(key)] });
			return store.get(keyToString(key)) ?? null;
		},
		getBatch: async (keys) => {
			stats.getBatchCalls++;
			stats.getBatchKeys += keys.length;
			log.push({ op: "getBatch", keys: keys.map(decodeKey) });
			return keys.map((key) => store.get(keyToString(key)) ?? null);
		},
		put: async (key, value) => {
			stats.putCalls++;
			stats.putKeys++;
			log.push({ op: "put", keys: [decodeKey(key)] });
			store.set(keyToString(key), new Uint8Array(value));
		},
		putBatch: async (entries) => {
			stats.putBatchCalls++;
			stats.putBatchKeys += entries.length;
			log.push({
				op: "putBatch",
				keys: entries.map(([k]) => decodeKey(k)),
			});
			for (const [key, value] of entries) {
				store.set(keyToString(key), new Uint8Array(value));
			}
		},
		deleteBatch: async (keys) => {
			stats.deleteBatchCalls++;
			log.push({ op: "deleteBatch", keys: keys.map(decodeKey) });
			for (const key of keys) {
				store.delete(keyToString(key));
			}
		},
	};

	const resetStats = () => {
		stats.getCalls = 0;
		stats.getKeys = 0;
		stats.getBatchCalls = 0;
		stats.getBatchKeys = 0;
		stats.putCalls = 0;
		stats.putKeys = 0;
		stats.putBatchCalls = 0;
		stats.putBatchKeys = 0;
		stats.deleteBatchCalls = 0;
		log.length = 0;
	};

	return { kvStore, stats, log, resetStats };
}

// -- Helpers --

function generateCreateTables(n: number): string[] {
	const stmts: string[] = [];
	for (let i = 0; i < n; i++) {
		stmts.push(
			`CREATE TABLE IF NOT EXISTS t${i} (id INTEGER PRIMARY KEY, val TEXT NOT NULL)`,
		);
	}
	return stmts;
}

function printStats(label: string, stats: KvStats) {
	const totalReads = stats.getCalls + stats.getBatchCalls;
	const totalWrites = stats.putCalls + stats.putBatchCalls;
	console.log(
		`[${label}] reads=${totalReads} (get=${stats.getCalls} getBatch=${stats.getBatchCalls}/${stats.getBatchKeys}keys) ` +
			`writes=${totalWrites} (put=${stats.putCalls} putBatch=${stats.putBatchCalls}/${stats.putBatchKeys}keys) ` +
			`deletes=${stats.deleteBatchCalls}`,
	);
}

/** Open a DB, create a counter table, seed it, do two writes to warm the pager cache, then reset stats. */
async function openAndWarm(
	name: string,
	kv: KvVfsOptions,
	resetStats: () => void,
) {
	const vfs = await createSqliteVfs();
	const db = await vfs.open(name, kv);
	await db.exec(`
		CREATE TABLE IF NOT EXISTS counter (
			id INTEGER PRIMARY KEY,
			count INTEGER NOT NULL DEFAULT 0
		)
	`);
	await db.exec("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
	await db.exec("UPDATE counter SET count = count + 1 WHERE id = 1");
	await db.exec("UPDATE counter SET count = count + 1 WHERE id = 1");
	resetStats();
	return { vfs, db };
}

// ============================================================
// RivetKit flow: raw db() onMigrate with SAVEPOINT
// ============================================================
// Simulates what the framework does: wraps onMigrate in
// SAVEPOINT rivetkit_migrate / RELEASE rivetkit_migrate.

describe("rivetkit raw onMigrate (SAVEPOINT-wrapped)", () => {
	it("COLD: single table with seed INSERT", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("rk-raw-cold-1", kvStore);
		resetStats();

		// Framework wraps in savepoint
		await db.exec("SAVEPOINT rivetkit_migrate");
		await db.exec(`
			CREATE TABLE IF NOT EXISTS test_data (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value TEXT NOT NULL,
				payload TEXT NOT NULL DEFAULT '',
				created_at INTEGER NOT NULL
			)
		`);
		await db.exec("RELEASE rivetkit_migrate");

		printStats("rivetkit raw COLD 1 table", stats);
		expect(stats.putBatchCalls).toBe(1);
		await db.close();
	});

	it("COLD: sandbox-style migration (2 tables + 2 indexes + DROP)", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("rk-raw-cold-sandbox", kvStore);
		resetStats();

		await db.exec("SAVEPOINT rivetkit_migrate");

		// Mirrors sandbox actor: db.ts migrateSandboxTables
		await db.exec(`
			DROP TABLE IF EXISTS sandbox_actor_meta;
			DROP TABLE IF EXISTS sandbox_actor_sessions;
		`);
		await db.exec(`
			CREATE TABLE IF NOT EXISTS sandbox_agent_sessions (
				id TEXT PRIMARY KEY,
				created_at INTEGER NOT NULL,
				record_json TEXT NOT NULL
			);
			CREATE TABLE IF NOT EXISTS sandbox_agent_events (
				id TEXT PRIMARY KEY,
				session_id TEXT NOT NULL,
				event_index INTEGER NOT NULL,
				created_at INTEGER NOT NULL,
				connection_id TEXT NOT NULL,
				sender TEXT NOT NULL,
				payload_json TEXT NOT NULL,
				raw_payload_json TEXT,
				UNIQUE(session_id, event_index)
			);
			CREATE INDEX IF NOT EXISTS sandbox_agent_sessions_created_at_idx
				ON sandbox_agent_sessions (created_at DESC);
			CREATE INDEX IF NOT EXISTS sandbox_agent_events_session_event_index_idx
				ON sandbox_agent_events (session_id, event_index ASC);
		`);

		await db.exec("RELEASE rivetkit_migrate");

		printStats("rivetkit raw COLD sandbox (2 tables + 2 indexes)", stats);
		expect(stats.putBatchCalls).toBe(1);
		await db.close();
	});

	it("COLD: DDL + seed INSERT OR IGNORE (cloudflare pattern)", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("rk-raw-cold-seed", kvStore);
		resetStats();

		await db.exec("SAVEPOINT rivetkit_migrate");
		await db.exec(`
			CREATE TABLE IF NOT EXISTS counter (
				id INTEGER PRIMARY KEY,
				count INTEGER NOT NULL DEFAULT 0
			)
		`);
		await db.exec("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
		await db.exec("RELEASE rivetkit_migrate");

		printStats("rivetkit raw COLD DDL + seed", stats);
		expect(stats.putBatchCalls).toBe(1);
		await db.close();
	});

	it("HOT: all patterns produce 0 writes on reopen", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();

		// Cold phase
		const vfs1 = await createSqliteVfs();
		const db1 = await vfs1.open("rk-raw-hot", kvStore);
		await db1.exec("SAVEPOINT rivetkit_migrate");
		await db1.exec(`
			CREATE TABLE IF NOT EXISTS test_data (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value TEXT NOT NULL
			)
		`);
		await db1.exec(
			"INSERT OR IGNORE INTO test_data (id, value) VALUES (1, 'seed')",
		);
		await db1.exec("RELEASE rivetkit_migrate");
		await db1.close();

		// Hot phase
		resetStats();
		const vfs2 = await createSqliteVfs();
		const db2 = await vfs2.open("rk-raw-hot", kvStore);
		resetStats();

		await db2.exec("SAVEPOINT rivetkit_migrate");
		await db2.exec(`
			CREATE TABLE IF NOT EXISTS test_data (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value TEXT NOT NULL
			)
		`);
		await db2.exec(
			"INSERT OR IGNORE INTO test_data (id, value) VALUES (1, 'seed')",
		);
		await db2.exec("RELEASE rivetkit_migrate");

		printStats("rivetkit raw HOT", stats);
		expect(stats.putBatchCalls).toBe(0);
		await db2.close();
	});
});

// ============================================================
// RivetKit flow: Drizzle onMigrate with per-migration SAVEPOINT
// ============================================================
// Simulates runInlineMigrations: creates __drizzle_migrations
// tracking table, then wraps each migration in its own savepoint.

describe("rivetkit drizzle onMigrate (per-migration SAVEPOINT)", () => {
	it("COLD: single migration with 1 table", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("rk-drizzle-cold-1", kvStore);
		resetStats();

		// Tracking table (not wrapped, matches framework behavior)
		await db.exec(`
			CREATE TABLE IF NOT EXISTS __drizzle_migrations (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				hash TEXT NOT NULL,
				created_at INTEGER
			)
		`);

		// Migration 0: wrapped in savepoint
		await db.exec("SAVEPOINT rivetkit_migrate");
		await db.exec(`
			CREATE TABLE IF NOT EXISTS test_data (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value TEXT NOT NULL,
				payload TEXT NOT NULL DEFAULT '',
				created_at INTEGER NOT NULL
			)
		`);
		await db.run(
			"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
			["0000_init", 1700000000000],
		);
		await db.exec("RELEASE rivetkit_migrate");

		printStats("rivetkit drizzle COLD 1 migration", stats);
		// Tracking table = 1 write, migration savepoint = 1 write
		expect(stats.putBatchCalls).toBe(2);
		await db.close();
	});

	it("COLD: 3 sequential migrations", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("rk-drizzle-cold-3", kvStore);
		resetStats();

		await db.exec(`
			CREATE TABLE IF NOT EXISTS __drizzle_migrations (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				hash TEXT NOT NULL,
				created_at INTEGER
			)
		`);

		const migrations = [
			{
				sql: "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
				hash: "0000_users",
				when: 1700000000000,
			},
			{
				sql: "CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, title TEXT NOT NULL)",
				hash: "0001_posts",
				when: 1700000001000,
			},
			{
				sql: "CREATE TABLE IF NOT EXISTS comments (id INTEGER PRIMARY KEY, post_id INTEGER NOT NULL, body TEXT NOT NULL)",
				hash: "0002_comments",
				when: 1700000002000,
			},
		];

		for (const m of migrations) {
			await db.exec("SAVEPOINT rivetkit_migrate");
			await db.exec(m.sql);
			await db.run(
				"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
				[m.hash, m.when],
			);
			await db.exec("RELEASE rivetkit_migrate");
		}

		printStats("rivetkit drizzle COLD 3 migrations", stats);
		// 1 for tracking table + 3 migration savepoints
		expect(stats.putBatchCalls).toBe(4);
		await db.close();
	});

	it("HOT: all migrations already applied, 0 writes", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();

		// Cold phase
		const vfs1 = await createSqliteVfs();
		const db1 = await vfs1.open("rk-drizzle-hot", kvStore);
		await db1.exec(`
			CREATE TABLE IF NOT EXISTS __drizzle_migrations (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				hash TEXT NOT NULL,
				created_at INTEGER
			)
		`);
		await db1.exec("SAVEPOINT rivetkit_migrate");
		await db1.exec(
			"CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
		);
		await db1.run(
			"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
			["0000_users", 1700000000000],
		);
		await db1.exec("RELEASE rivetkit_migrate");
		await db1.close();

		// Hot phase: reopen, migrations already applied
		resetStats();
		const vfs2 = await createSqliteVfs();
		const db2 = await vfs2.open("rk-drizzle-hot", kvStore);
		resetStats();

		// Tracking table CREATE IF NOT EXISTS is a no-op
		await db2.exec(`
			CREATE TABLE IF NOT EXISTS __drizzle_migrations (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				hash TEXT NOT NULL,
				created_at INTEGER
			)
		`);

		// Check last applied migration
		let lastCreatedAt = 0;
		await db2.exec(
			"SELECT id, hash, created_at FROM __drizzle_migrations ORDER BY created_at DESC LIMIT 1",
			(row) => {
				lastCreatedAt = Number(row[2]) || 0;
			},
		);

		// Migration already applied, skip
		const migrationWhen = 1700000000000;
		if (migrationWhen > lastCreatedAt) {
			throw new Error("migration should have been skipped");
		}

		printStats("rivetkit drizzle HOT (skip)", stats);
		expect(stats.putBatchCalls).toBe(0);
		await db2.close();
	});
});

// ============================================================
// Experimental: raw exec() batching behavior
// ============================================================
// These tests isolate the VFS batching behavior at the exec()
// level without the framework SAVEPOINT wrapper, to verify how
// SQLite batches writes with different call patterns.

const TABLE_COUNTS = [1, 5, 20, 100];

describe("experimental: exec() batching without SAVEPOINT", () => {
	for (const n of TABLE_COUNTS) {
		describe(`${n} CREATE TABLE statements`, () => {
			it(`COLD separate exec() calls`, async () => {
				const { kvStore, stats, resetStats } =
					createInstrumentedKvStore();
				const vfs = await createSqliteVfs();
				const db = await vfs.open(`exp-sep-${n}`, kvStore);
				resetStats();

				for (const stmt of generateCreateTables(n)) {
					await db.exec(stmt);
				}

				printStats(`exp COLD ${n}x separate exec()`, stats);
				expect(stats.putBatchCalls).toBe(n);
				await db.close();
			});

			it(`COLD single multi-statement exec()`, async () => {
				const { kvStore, stats, resetStats } =
					createInstrumentedKvStore();
				const vfs = await createSqliteVfs();
				const db = await vfs.open(`exp-single-${n}`, kvStore);
				resetStats();

				await db.exec(generateCreateTables(n).join(";\n") + ";");

				printStats(`exp COLD ${n}x single exec()`, stats);
				expect(stats.putBatchCalls).toBe(n);
				await db.close();
			});

			it(`COLD BEGIN/COMMIT wrapped`, async () => {
				const { kvStore, stats, resetStats } =
					createInstrumentedKvStore();
				const vfs = await createSqliteVfs();
				const db = await vfs.open(`exp-tx-${n}`, kvStore);
				resetStats();

				const stmts = generateCreateTables(n);
				await db.exec(
					`BEGIN;\n${stmts.join(";\n")};\nCOMMIT;`,
				);

				printStats(`exp COLD ${n}x BEGIN/COMMIT`, stats);
				expect(stats.putBatchCalls).toBe(1);
				await db.close();
			});

			it(`COLD SAVEPOINT wrapped`, async () => {
				const { kvStore, stats, resetStats } =
					createInstrumentedKvStore();
				const vfs = await createSqliteVfs();
				const db = await vfs.open(`exp-sp-${n}`, kvStore);
				resetStats();

				const stmts = generateCreateTables(n);
				await db.exec("SAVEPOINT sp");
				for (const stmt of stmts) {
					await db.exec(stmt);
				}
				await db.exec("RELEASE sp");

				printStats(`exp COLD ${n}x SAVEPOINT`, stats);
				expect(stats.putBatchCalls).toBe(1);
				await db.close();
			});

			it(`HOT: all patterns produce 0 writes`, async () => {
				const { kvStore, stats, resetStats } =
					createInstrumentedKvStore();

				const vfs1 = await createSqliteVfs();
				const db1 = await vfs1.open(`exp-hot-${n}`, kvStore);
				await db1.exec(
					generateCreateTables(n).join(";\n") + ";",
				);
				await db1.close();

				resetStats();
				const vfs2 = await createSqliteVfs();
				const db2 = await vfs2.open(`exp-hot-${n}`, kvStore);
				resetStats();

				for (const stmt of generateCreateTables(n)) {
					await db2.exec(stmt);
				}

				printStats(`exp HOT ${n}x`, stats);
				expect(stats.putBatchCalls).toBe(0);
				await db2.close();
			});
		});
	}
});

// ============================================================
// Warm path: steady-state KV behavior after migration
// ============================================================

describe("warm path: KV behavior after migration", () => {
	it("UPDATE uses BATCH_ATOMIC: exactly 1 putBatch, 0 reads, no journal", async () => {
		const { kvStore, stats, log, resetStats } =
			createInstrumentedKvStore();
		const { db } = await openAndWarm("warm-update", kvStore, resetStats);

		await db.exec("UPDATE counter SET count = count + 1 WHERE id = 1");

		expect(stats.putBatchCalls).toBe(1);
		expect(stats.getBatchCalls).toBe(0);

		const allKeys = log.flatMap((e) => e.keys);
		const journalKeys = allKeys.filter((k) => k.includes("journal"));
		expect(journalKeys.length).toBe(0);

		await db.close();
	});

	it("SELECT uses 0 KV round trips", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const { db } = await openAndWarm("warm-select", kvStore, resetStats);

		await db.query("SELECT count FROM counter WHERE id = 1");

		expect(stats.getBatchCalls).toBe(0);
		expect(stats.putBatchCalls).toBe(0);

		await db.close();
	});

	it("SELECT after UPDATE adds no extra KV round trips", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const { db } = await openAndWarm(
			"warm-select-after-update",
			kvStore,
			resetStats,
		);

		await db.exec("UPDATE counter SET count = count + 1 WHERE id = 1");
		const writeAfterUpdate = stats.putBatchCalls;
		const readAfterUpdate = stats.getBatchCalls;

		await db.query("SELECT count FROM counter WHERE id = 1");

		expect(stats.putBatchCalls).toBe(writeAfterUpdate);
		expect(stats.getBatchCalls).toBe(readAfterUpdate);

		await db.close();
	});

	it("multi-page INSERT writes multiple chunk keys", async () => {
		const { kvStore, stats, log, resetStats } =
			createInstrumentedKvStore();
		const { db } = await openAndWarm(
			"warm-multi-page",
			kvStore,
			resetStats,
		);

		await db.exec(`
			CREATE TABLE IF NOT EXISTS indexed_data (
				id INTEGER PRIMARY KEY,
				value TEXT NOT NULL
			)
		`);
		await db.exec(
			"CREATE INDEX IF NOT EXISTS idx_indexed_data_value ON indexed_data(value)",
		);
		resetStats();

		await db.run("INSERT INTO indexed_data (value) VALUES (?)", [
			`row-${Date.now()}`,
		]);

		expect(stats.putBatchCalls).toBeGreaterThanOrEqual(1);
		expect(stats.putBatchKeys).toBeGreaterThan(1);

		const putOps = log.filter(
			(e) => e.op === "putBatch" || e.op === "put",
		);
		const mainChunkKeys = putOps
			.flatMap((e) => e.keys)
			.filter((k) => k.startsWith("chunk:main["));
		expect(mainChunkKeys.length).toBeGreaterThanOrEqual(1);

		await db.close();
	});

	it("ROLLBACK produces no data page writes", async () => {
		const { kvStore, log, resetStats } = createInstrumentedKvStore();
		const { db } = await openAndWarm(
			"warm-rollback",
			kvStore,
			resetStats,
		);

		await db.exec(`
			CREATE TABLE IF NOT EXISTS rollback_test (
				id INTEGER PRIMARY KEY,
				value TEXT NOT NULL
			)
		`);
		resetStats();

		await db.exec(`
			BEGIN;
			INSERT INTO rollback_test (value) VALUES ('should-not-persist');
			ROLLBACK;
		`);

		const putOps = log.filter(
			(e) => e.op === "putBatch" || e.op === "put",
		);
		const mainChunkKeys = putOps
			.flatMap((e) => e.keys)
			.filter((k) => k.startsWith("chunk:main["));
		expect(mainChunkKeys.length).toBe(0);

		await db.close();
	});

	it("multi-statement transaction produces writes", async () => {
		const { kvStore, stats, resetStats } = createInstrumentedKvStore();
		const { db } = await openAndWarm(
			"warm-multi-stmt-tx",
			kvStore,
			resetStats,
		);

		await db.exec(`
			CREATE TABLE IF NOT EXISTS multi_stmt (
				id INTEGER PRIMARY KEY,
				value TEXT NOT NULL
			)
		`);
		resetStats();

		await db.exec(`
			BEGIN;
			INSERT INTO multi_stmt (value) VALUES ('row-a');
			INSERT INTO multi_stmt (value) VALUES ('row-b');
			COMMIT;
		`);

		expect(stats.putBatchCalls).toBeGreaterThanOrEqual(1);

		await db.close();
	});
});

// ============================================================
// Structural properties: invariants regardless of cache state
// ============================================================

describe("structural properties", () => {
	it("no WAL or SHM operations occur", async () => {
		const { kvStore, log, resetStats } = createInstrumentedKvStore();
		const { db } = await openAndWarm("no-wal-shm", kvStore, resetStats);

		await db.exec("UPDATE counter SET count = count + 1 WHERE id = 1");

		const allKeys = log.flatMap((e) => e.keys);
		const walOrShmKeys = allKeys.filter(
			(k) => k.includes("wal") || k.includes("shm"),
		);
		expect(walOrShmKeys.length).toBe(0);

		await db.close();
	});

	it("every putBatch has at most 128 keys", async () => {
		const { kvStore, log, resetStats } = createInstrumentedKvStore();
		const { db } = await openAndWarm(
			"putbatch-limit",
			kvStore,
			resetStats,
		);

		await db.exec("UPDATE counter SET count = count + 1 WHERE id = 1");

		const putBatchOps = log.filter((e) => e.op === "putBatch");
		for (const entry of putBatchOps) {
			expect(entry.keys.length).toBeLessThanOrEqual(128);
		}

		await db.close();
	});
});

// ============================================================
// Large transactions: journal fallback and data integrity
// ============================================================

describe("large transactions", () => {
	it("falls back to journal when exceeding 127 dirty pages", async () => {
		const { kvStore, stats, log, resetStats } =
			createInstrumentedKvStore();
		const { db } = await openAndWarm(
			"large-tx-journal",
			kvStore,
			resetStats,
		);

		await db.exec(`
			CREATE TABLE IF NOT EXISTS bulk_data (
				id INTEGER PRIMARY KEY,
				payload TEXT NOT NULL
			)
		`);
		resetStats();

		const pad = "x".repeat(4000);
		const stmts = ["BEGIN;"];
		for (let i = 0; i < 200; i++) {
			stmts.push(
				`INSERT INTO bulk_data (payload) VALUES ('bulk-${i}-${pad}');`,
			);
		}
		stmts.push("COMMIT;");
		await db.exec(stmts.join("\n"));

		expect(stats.putBatchCalls).toBeGreaterThan(1);

		const allKeys = log.flatMap((e) => e.keys);
		const journalKeys = allKeys.filter((k) => k.includes("journal"));
		expect(journalKeys.length).toBeGreaterThan(0);

		const putBatchOps = log.filter((e) => e.op === "putBatch");
		for (const entry of putBatchOps) {
			expect(entry.keys.length).toBeLessThanOrEqual(128);
		}

		await db.close();
	});

	it("data integrity: 200 rows and integrity check pass", async () => {
		const { kvStore, resetStats } = createInstrumentedKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("large-tx-integrity", kvStore);
		resetStats();

		await db.exec(`
			CREATE TABLE IF NOT EXISTS bulk_data (
				id INTEGER PRIMARY KEY,
				payload TEXT NOT NULL
			)
		`);

		const pad = "x".repeat(4000);
		const stmts = ["BEGIN;"];
		for (let i = 0; i < 200; i++) {
			stmts.push(
				`INSERT INTO bulk_data (payload) VALUES ('bulk-${i}-${pad}');`,
			);
		}
		stmts.push("COMMIT;");
		await db.exec(stmts.join("\n"));

		const countResult = await db.query(
			"SELECT COUNT(*) as cnt FROM bulk_data",
		);
		expect(Number(countResult.rows[0]?.[0])).toBe(200);

		const integrityResult = await db.query("PRAGMA integrity_check");
		expect(String(integrityResult.rows[0]?.[0]).toLowerCase()).toBe("ok");

		await db.close();
	});

	it("survives close and reopen", async () => {
		const { kvStore, resetStats } = createInstrumentedKvStore();

		const vfs1 = await createSqliteVfs();
		const db1 = await vfs1.open("large-tx-reopen", kvStore);

		await db1.exec(`
			CREATE TABLE IF NOT EXISTS bulk_data (
				id INTEGER PRIMARY KEY,
				payload TEXT NOT NULL
			)
		`);

		const pad = "x".repeat(4000);
		const stmts = ["BEGIN;"];
		for (let i = 0; i < 200; i++) {
			stmts.push(
				`INSERT INTO bulk_data (payload) VALUES ('bulk-${i}-${pad}');`,
			);
		}
		stmts.push("COMMIT;");
		await db1.exec(stmts.join("\n"));

		const countBefore = await db1.query(
			"SELECT COUNT(*) as cnt FROM bulk_data",
		);
		expect(Number(countBefore.rows[0]?.[0])).toBe(200);
		await db1.close();

		resetStats();
		const vfs2 = await createSqliteVfs();
		const db2 = await vfs2.open("large-tx-reopen", kvStore);

		const countAfter = await db2.query(
			"SELECT COUNT(*) as cnt FROM bulk_data",
		);
		expect(Number(countAfter.rows[0]?.[0])).toBe(200);

		const integrityResult = await db2.query("PRAGMA integrity_check");
		expect(String(integrityResult.rows[0]?.[0]).toLowerCase()).toBe("ok");

		await db2.close();
	});
});
