import { createHash, randomBytes } from "node:crypto";
import { actor } from "rivetkit";
import { db } from "rivetkit/db";

// Seeding vessel for the "large uncompacted SQLite DB" load test.
//
// The old engine build under test has hot compaction disabled, so every
// commit accumulates as an uncompacted delta chain in FDB that never folds.
// This actor grows a persistent SQLite database with append-only inserts and
// one commit per batch, producing both large page volume and many deltas.
//
// It deliberately avoids VACUUM, DELETE, and whole-database scans (no
// integrity_check) so the database only ever grows and each grow() call costs
// O(added bytes), not O(current size).

interface GrowInput {
	// Target total on-disk size in mebibytes. grow() is idempotent to this
	// target: calling it again after the target is reached is a cheap no-op.
	targetMb?: number;
	// Rows inserted per committed batch. Each batch is one BEGIN/COMMIT, so
	// this controls commit (delta) granularity.
	batchRows?: number;
	// Payload bytes per row, generated server-side via randomblob().
	rowBytes?: number;
	// Wall-clock budget for a single grow() call in milliseconds. When the
	// budget is exceeded the call returns done=false so the driver can resume
	// with another call. Keeps a single action under the action timeout.
	budgetMs?: number;
}

interface AppendInput {
	rows?: number;
	rowBytes?: number;
}

interface AppendResult {
	rows: number;
	rowBytes: number;
}

interface ReadRandomInput {
	count?: number;
}

interface ReadRandomResult {
	count: number;
	hits: number;
	bytes: number;
	maxId: number;
}

interface DigestInput {
	// Hash only rows with an id above this one.
	afterId?: number;
	// Hash only rows with an id up to this one, so rows appended later are left out.
	maxId?: number;
	// Rows hashed per call. Bounds one call's time and memory on any database size.
	limit?: number;
	// The chain value after the row `afterId`, as returned by the previous call. Hex; empty or
	// absent for the start of the table.
	chain?: string;
}

interface DigestResult {
	rows: number;
	// The id of the last hashed row, where the next call continues. `afterId` when no row was hashed.
	lastId: number;
	// The chain after the last hashed row.
	chain: string;
}

interface StoredDigest {
	// The chain over every row grow() inserted, kept in the same transaction as each batch.
	chain: string;
	rows: number;
	lastId: number;
}

interface StorageStats {
	pageCount: number;
	pageSize: number;
	freelistCount: number;
	sizeBytes: number;
}

interface GrowResult extends StorageStats {
	done: boolean;
	targetBytes: number;
	batchesThisCall: number;
	rowsThisCall: number;
	elapsedMs: number;
}

const DEFAULT_DIGEST_LIMIT = 2048;
const CHAIN_START = "0".repeat(64);

// The content chain over rows in id order: each row's hash is folded into
// the previous chain value. grow() keeps it current as it inserts, so the
// expected content of a database is one small row instead of a full read.
function chainRow(
	chain: string,
	row: { id: number; batch: number; created_at: number; payload: Uint8Array },
): string {
	const rowHash = createHash("sha256")
		.update(`${row.id}:${row.batch}:${row.created_at}:`)
		.update(row.payload)
		.digest();
	return createHash("sha256")
		.update(Buffer.from(chain, "hex"))
		.update(rowHash)
		.digest("hex");
}

async function storedDigest(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}): Promise<StoredDigest> {
	const rows = (await database.execute(
		"SELECT chain, rows, last_id FROM grow_digest WHERE id = 1",
	)) as Array<{ chain: string; rows: number; last_id: number }>;
	const row = rows[0];
	if (!row) return { chain: CHAIN_START, rows: 0, lastId: 0 };
	return { chain: row.chain, rows: row.rows, lastId: row.last_id };
}
const DEFAULT_TARGET_MB = 64;
// 64 rows x 8 KiB payload dirties ~130 SQLite pages per commit, comfortably
// under the depot MAX_COMMIT_DIRTY_PAGES (320) cap and giving one uncompacted
// FDB delta per batch.
const DEFAULT_BATCH_ROWS = 64;
const DEFAULT_ROW_BYTES = 8 * 1024;
const DEFAULT_BUDGET_MS = 120_000;
const MEBIBYTE = 1024 * 1024;

function finiteInt(value: number | undefined, fallback: number): number {
	if (value === undefined) return fallback;
	if (!Number.isFinite(value) || value <= 0) {
		throw new Error(`expected a positive finite number, got ${value}`);
	}
	return Math.floor(value);
}

async function queryOne<T>(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	sql: string,
): Promise<T> {
	const rows = await database.execute(sql);
	if (!rows[0]) throw new Error(`query returned no rows: ${sql}`);
	return rows[0] as T;
}

async function storageStats(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}): Promise<StorageStats> {
	const [pageCount, pageSize, freelistCount] = await Promise.all([
		queryOne<{ page_count: number }>(database, "PRAGMA page_count"),
		queryOne<{ page_size: number }>(database, "PRAGMA page_size"),
		queryOne<{ freelist_count: number }>(database, "PRAGMA freelist_count"),
	]);
	return {
		pageCount: pageCount.page_count,
		pageSize: pageSize.page_size,
		freelistCount: freelistCount.freelist_count,
		sizeBytes: pageCount.page_count * pageSize.page_size,
	};
}

export const growDb = actor({
	options: {
		// Generous per-action timeout. grow() self-limits with budgetMs and
		// returns done=false before this fires.
		actionTimeout: 1_800_000,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS grow_rows (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					batch INTEGER NOT NULL,
					payload BLOB NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS grow_digest (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					chain TEXT NOT NULL,
					rows INTEGER NOT NULL,
					last_id INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		grow: async (c, input: GrowInput = {}): Promise<GrowResult> => {
			const startedAt = performance.now();
			const targetMb = finiteInt(input.targetMb, DEFAULT_TARGET_MB);
			const batchRows = finiteInt(input.batchRows, DEFAULT_BATCH_ROWS);
			const rowBytes = finiteInt(input.rowBytes, DEFAULT_ROW_BYTES);
			const budgetMs = finiteInt(input.budgetMs, DEFAULT_BUDGET_MS);
			const targetBytes = targetMb * MEBIBYTE;

			const placeholders = new Array(batchRows)
				.fill("(?, ?, ?, ?)")
				.join(", ");

			let batchesThisCall = 0;
			let rowsThisCall = 0;
			let stats = await storageStats(c.db);

			while (stats.sizeBytes < targetBytes) {
				if (performance.now() - startedAt >= budgetMs) {
					return {
						...stats,
						done: false,
						targetBytes,
						batchesThisCall,
						rowsThisCall,
						elapsedMs: Math.round(performance.now() - startedAt),
					};
				}

				// One commit per batch so each iteration produces exactly one
				// uncompacted delta in FDB. The content chain advances in the
				// same transaction, so it always covers exactly the stored rows.
				await c.db.execute("BEGIN");
				try {
					const digest = await storedDigest(c.db);
					const maxRow = await queryOne<{ max_id: number | null }>(
						c.db,
						"SELECT max(id) AS max_id FROM grow_rows",
					);
					if ((maxRow.max_id ?? 0) !== digest.lastId) {
						throw new Error(
							`grow_digest covers rows up to ${digest.lastId} but the table holds rows up to ${maxRow.max_id}`,
						);
					}
					const now = Date.now();
					const args: unknown[] = [];
					let chain = digest.chain;
					for (let i = 0; i < batchRows; i += 1) {
						const row = {
							id: digest.lastId + 1 + i,
							batch: batchesThisCall,
							created_at: now,
							payload: randomBytes(rowBytes),
						};
						chain = chainRow(chain, row);
						args.push(row.id, row.batch, row.payload, row.created_at);
					}
					await c.db.execute(
						`INSERT INTO grow_rows (id, batch, payload, created_at) VALUES ${placeholders}`,
						...args,
					);
					await c.db.execute(
						"INSERT INTO grow_digest (id, chain, rows, last_id) VALUES (1, ?, ?, ?) ON CONFLICT (id) DO UPDATE SET chain = excluded.chain, rows = excluded.rows, last_id = excluded.last_id",
						chain,
						digest.rows + batchRows,
						digest.lastId + batchRows,
					);
					await c.db.execute("COMMIT");
				} catch (err) {
					await c.db.execute("ROLLBACK").catch(() => undefined);
					throw err;
				}

				batchesThisCall += 1;
				rowsThisCall += batchRows;
				stats = await storageStats(c.db);
			}

			return {
				...stats,
				done: true,
				targetBytes,
				batchesThisCall,
				rowsThisCall,
				elapsedMs: Math.round(performance.now() - startedAt),
			};
		},
		stats: async (c): Promise<StorageStats> => storageStats(c.db),
		// One small committed insert, used by the live-traffic driver to imitate
		// steady production writes against an already seeded database. Unlike
		// grow() it never loops toward a size target, so each call is exactly
		// one commit (one FDB delta) regardless of the current size.
		append: async (c, input: AppendInput = {}): Promise<AppendResult> => {
			const rows = finiteInt(input.rows, 1);
			const rowBytes = finiteInt(input.rowBytes, DEFAULT_ROW_BYTES);
			const placeholders = new Array(rows)
				.fill("(?, randomblob(?), ?)")
				.join(", ");
			const now = Date.now();
			const args: unknown[] = [];
			for (let i = 0; i < rows; i += 1) {
				args.push(-1, rowBytes, now);
			}
			// A single autocommit statement rather than BEGIN/COMMIT: append() calls
			// on one actor run concurrently, and an explicit BEGIN while another
			// append's transaction is open fails with a nested-transaction error.
			await c.db.execute(
				`INSERT INTO grow_rows (batch, payload, created_at) VALUES ${placeholders}`,
				...args,
			);
			return { rows, rowBytes };
		},
		// Point reads of random rows, used by the live-traffic driver to imitate
		// production reads. Each row lives on its own overflow pages, so every
		// hit pulls a handful of pages through the depot VFS instead of a
		// whole-table scan.
		readRandom: async (
			c,
			input: ReadRandomInput = {},
		): Promise<ReadRandomResult> => {
			const count = finiteInt(input.count, 1);
			const maxRow = await queryOne<{ max_id: number | null }>(
				c.db,
				"SELECT max(id) AS max_id FROM grow_rows",
			);
			const maxId = maxRow.max_id ?? 0;
			let bytes = 0;
			let hits = 0;
			for (let i = 0; i < count; i += 1) {
				const id =
					maxId === 0 ? 0 : 1 + Math.floor(Math.random() * maxId);
				const rows = (await c.db.execute(
					"SELECT length(payload) AS n FROM grow_rows WHERE id = ?",
					id,
				)) as Array<{ n: number }>;
				if (rows[0]) {
					hits += 1;
					bytes += rows[0].n;
				}
			}
			return { count, hits, bytes, maxId };
		},
		// Content verification for the migration load test. The caller chains
		// the hashes of consecutive ranges, so a database of any size is hashed
		// in calls that each stay well under the action and gateway timeouts.
		digest: async (c, input: DigestInput = {}): Promise<DigestResult> => {
			const afterId = input.afterId ?? 0;
			const maxId = input.maxId ?? Number.MAX_SAFE_INTEGER;
			const limit = finiteInt(input.limit, DEFAULT_DIGEST_LIMIT);
			const rows = (await c.db.execute(
				"SELECT id, batch, created_at, payload FROM grow_rows WHERE id > ? AND id <= ? ORDER BY id LIMIT ?",
				afterId,
				maxId,
				limit,
			)) as Array<{
				id: number;
				batch: number;
				created_at: number;
				payload: Uint8Array | ArrayBuffer;
			}>;
			let chain = input.chain || CHAIN_START;
			for (const row of rows) {
				chain = chainRow(chain, { ...row, payload: new Uint8Array(row.payload) });
			}
			return { rows: rows.length, lastId: rows.at(-1)?.id ?? afterId, chain };
		},
		// The chain grow() kept: the expected content of the database, read in
		// one query.
		storedDigest: async (c): Promise<StoredDigest> => storedDigest(c.db),
		// Full-database verification for the compaction rig. This reads every
		// page through the depot VFS, so it exercises PIDX resolution across
		// folded shards and any remaining unfolded deltas. Deliberately not
		// called during seeding, where it would cost O(current size) per call.
		integrityCheck: async (
			c,
		): Promise<
			{ ok: boolean; result: string; rows: number } & StorageStats
		> => {
			const rows = (await c.db.execute(
				"PRAGMA integrity_check",
			)) as Array<Record<string, unknown>>;
			const messages = rows.map((row) => String(Object.values(row)[0]));
			const result = messages.join("; ");
			const counted = (await c.db.execute(
				"SELECT count(*) AS n FROM grow_rows",
			)) as Array<{ n: number }>;
			return {
				...(await storageStats(c.db)),
				ok: messages.length === 1 && messages[0] === "ok",
				result,
				rows: counted[0]?.n ?? -1,
			};
		},
	},
});
