import { actor } from "rivetkit";
import { db } from "rivetkit/db";

// Exercise vessel for depot's segmented staged commits.
//
// Depot sends a commit of at most 320 dirty pages as one message applied in one
// FDB transaction, and anything larger as N shard-aligned staged segments
// followed by a finalize. This actor's job is to put commits on both sides of
// that boundary on demand, so the staged path can be checked for the things
// that only go wrong at scale: pages resolving to the wrong version, a commit
// that half-lands, and what a large commit does to write throughput.
//
// Payloads are deterministic in the row id rather than random, so correctness
// is checkable in SQL over the whole table without reading it back over the
// wire. `integrity_check` alone would not catch a page that resolves to a
// stale-but-valid version, which is the failure mode segmented folds risk.

interface WriteInput {
	// Rows in the single transaction this call commits. Page count lands near
	// rows * ceil(rowBytes / pageSize), so this is the dial that chooses the
	// single-shot path or the staged one.
	rows?: number;
	// Payload bytes per row. Kept above the page size so one row dirties at
	// least one page and the page count stays predictable from the row count.
	rowBytes?: number;
	// Split the same rows across this many transactions instead of one. The
	// equivalence test writes identical content with 1 and with many, so the
	// staged path can be compared against the single-shot path it replaces.
	batches?: number;
	// Fill rows with random bytes instead of the deterministic payload.
	//
	// The deterministic payload is a repeating id, which compresses by around
	// forty times, so a commit that is a hundred megabytes of pages is only a
	// couple of megabytes once staged. That is fine for correctness and useless
	// for anything measuring byte volume: a throttle sees the compressed size.
	// Random rows make a commit cost what its page count implies, at the price
	// of losing per-row verification, which `verify` falls back to length and
	// row counting for.
	random?: boolean;
}

interface StorageStats {
	pageCount: number;
	pageSize: number;
	freelistCount: number;
	sizeBytes: number;
	rows: number;
}

interface WriteResult extends StorageStats {
	pagesBefore: number;
	pagesDirtied: number;
	batches: number;
	rowsWritten: number;
	elapsedMs: number;
	// Wall time of the slowest single transaction, which is what a throttle
	// delay shows up in.
	slowestBatchMs: number;
}

const DEFAULT_ROWS = 1_000;
// One row per page keeps `rows` a direct dial on dirty pages. 4096 exactly would
// leave no room for the row header, so the row spills to a second page; 4000
// keeps one row to one page with the overhead included.
const DEFAULT_ROW_BYTES = 4_000;

function finiteInt(value: number | undefined, fallback: number): number {
	if (value === undefined) return fallback;
	if (!Number.isFinite(value) || value <= 0) {
		throw new Error(`expected a positive finite number, got ${value}`);
	}
	return Math.floor(value);
}

// The payload every row is expected to hold, as a SQL expression over `id`.
// Repeating a zero-padded id fills the row deterministically and lets the
// verifier recompute the same bytes without shipping them anywhere.
function randomPayloadExpr(rowBytes: number): string {
	return `substr(hex(randomblob(${Math.ceil(rowBytes / 2)})), 1, ${rowBytes})`;
}

function payloadExpr(idExpr: string, rowBytes: number): string {
	return `substr(replace(hex(zeroblob(${Math.ceil(rowBytes / 8)})), '00', printf('%08d', ${idExpr})), 1, ${rowBytes})`;
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
	const [pageCount, pageSize, freelistCount, rows] = await Promise.all([
		queryOne<{ page_count: number }>(database, "PRAGMA page_count"),
		queryOne<{ page_size: number }>(database, "PRAGMA page_size"),
		queryOne<{ freelist_count: number }>(database, "PRAGMA freelist_count"),
		queryOne<{ n: number }>(database, "SELECT count(*) AS n FROM big_rows"),
	]);
	return {
		pageCount: pageCount.page_count,
		pageSize: pageSize.page_size,
		freelistCount: freelistCount.freelist_count,
		sizeBytes: pageCount.page_count * pageSize.page_size,
		rows: rows.n,
	};
}

export const largeCommitDb = actor({
	options: {
		// A commit at the top of the range crosses the tunnel as a couple of
		// hundred staged segments, each its own round trip, so the write calls
		// need far more headroom than an ordinary action.
		actionTimeout: 600_000,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS big_rows (
					id INTEGER PRIMARY KEY,
					payload TEXT NOT NULL
				)
			`);
		},
	}),
	actions: {
		// Fairness scanner table: 116,480 4-KiB payloads occupy roughly 512 MiB with
		// SQLite leaf/overflow pages. Each call adds at most 8 MiB, so retries
		// resume from committed rows and never resend the database over HTTP.
		seedFairnessScan: async (c) => {
			await c.db.execute(
				"CREATE TABLE IF NOT EXISTS bench_rows (id INTEGER PRIMARY KEY, payload TEXT NOT NULL, revision INTEGER NOT NULL)",
			);
			const before = await queryOne<{ n: number }>(
				c.db,
				"SELECT coalesce(max(id), 0) AS n FROM bench_rows",
			);
			const target = Math.min(116_480, before.n + 2048);
			if (target > before.n) {
				await c.db.execute(
					`INSERT INTO bench_rows (id, payload, revision)
					 WITH RECURSIVE seq(id) AS (
					   SELECT ? UNION ALL SELECT id + 1 FROM seq WHERE id < ?
					 ) SELECT id, replace(hex(zeroblob(2048)), '0', 'x'), 0 FROM seq`,
					before.n + 1,
					target,
				);
			}
			return {
				...(await storageStats(c.db)),
				scanRows: target,
				complete: target === 116_480,
			};
		},

		// Compare every byte inside SQLite. Length-only scans can skip overflow
		// contents; full equality consumes them while returning only scalars.
		// The same actor owns both tables, so both scans share placement.
		scanFairness: async (c, kind: "large-row" | "table") => {
			if (kind !== "large-row" && kind !== "table") {
				throw new Error("expected large-row or table scanner");
			}
			const started = performance.now();
			const table = kind === "large-row" ? "big_rows" : "bench_rows";
			const expected =
				kind === "large-row"
					? payloadExpr("id", 64 * 1024 * 1024)
					: "replace(hex(zeroblob(2048)), '0', 'x')";
			const result = await queryOne<{
				rows: number;
				bytes: number;
				mismatches: number;
			}>(
				c.db,
				`SELECT count(*) AS rows, coalesce(sum(length(payload)), 0) AS bytes,
				 coalesce(sum(CASE WHEN payload = ${expected} THEN 0 ELSE 1 END), 0) AS mismatches
				 FROM ${table} NOT INDEXED`,
			);
			const expectedRows = kind === "large-row" ? 1 : 116_480;
			const expectedBytes =
				kind === "large-row" ? 64 * 1024 * 1024 : 116_480 * 4096;
			return {
				...result,
				ok:
					result.rows === expectedRows &&
					result.bytes === expectedBytes &&
					result.mismatches === 0,
				elapsedMs: performance.now() - started,
			};
		},
		// Appends `rows` deterministic rows, in `batches` transactions.
		//
		// With batches=1 this is the whole point of the actor: one commit whose
		// dirty page count is chosen by the caller, which depot then has to
		// stage in segments once it passes 320.
		write: async (c, input: WriteInput = {}): Promise<WriteResult> => {
			const startedAt = performance.now();
			const rows = finiteInt(input.rows, DEFAULT_ROWS);
			const rowBytes = finiteInt(input.rowBytes, DEFAULT_ROW_BYTES);
			const batches = finiteInt(input.batches, 1);
			const payload = input.random
				? randomPayloadExpr(rowBytes)
				: payloadExpr("id", rowBytes);
			if (rows % batches !== 0) {
				throw new Error(
					`rows ${rows} must divide evenly into ${batches} batches`,
				);
			}

			const before = await storageStats(c.db);
			const firstId = before.rows + 1;
			const perBatch = rows / batches;
			let slowestBatchMs = 0;

			for (let batch = 0; batch < batches; batch += 1) {
				const batchStart = firstId + batch * perBatch;
				const batchStartedAt = performance.now();
				await c.db.execute("BEGIN");
				try {
					// Generated in SQL rather than bound from JS so the payload
					// bytes never cross the wire. The write volume under test is
					// depot's, and shipping megabytes of parameters would just
					// measure the tunnel instead.
					await c.db.execute(
						`INSERT INTO big_rows (id, payload)
						 WITH RECURSIVE seq(id) AS (
							 SELECT ?
							 UNION ALL SELECT id + 1 FROM seq WHERE id < ?
						 )
						 SELECT id, ${payload} FROM seq`,
						batchStart,
						batchStart + perBatch - 1,
					);
					await c.db.execute("COMMIT");
				} catch (err) {
					await c.db.execute("ROLLBACK").catch(() => undefined);
					throw err;
				}
				slowestBatchMs = Math.max(
					slowestBatchMs,
					performance.now() - batchStartedAt,
				);
			}

			const after = await storageStats(c.db);
			return {
				...after,
				pagesBefore: before.pageCount,
				pagesDirtied: after.pageCount - before.pageCount,
				batches,
				rowsWritten: rows,
				elapsedMs: Math.round(performance.now() - startedAt),
				slowestBatchMs: Math.round(slowestBatchMs),
			};
		},

		// Rewrites every existing row in one transaction, which dirties pages
		// already in the database rather than appending new ones.
		//
		// Appending only ever stages pages at the end of the page space. An
		// overwrite scatters the dirty set across the whole file, so the commit
		// cuts into many more segments and finalize rewrites PIDX rows that
		// already had owners. That is the shape a fold can get wrong.
		rewriteAll: async (c, input: WriteInput = {}): Promise<WriteResult> => {
			const startedAt = performance.now();
			const rowBytes = finiteInt(input.rowBytes, DEFAULT_ROW_BYTES);
			const payload = input.random
				? randomPayloadExpr(rowBytes)
				: payloadExpr("id", rowBytes);
			const before = await storageStats(c.db);

			const batchStartedAt = performance.now();
			await c.db.execute("BEGIN");
			try {
				// Same expression the writer used, so a rewrite is a no-op in
				// content and any difference the verifier sees afterwards came
				// from the storage layer rather than from the SQL.
				await c.db.execute(`UPDATE big_rows SET payload = ${payload}`);
				await c.db.execute("COMMIT");
			} catch (err) {
				await c.db.execute("ROLLBACK").catch(() => undefined);
				throw err;
			}
			const slowestBatchMs = Math.round(
				performance.now() - batchStartedAt,
			);

			const after = await storageStats(c.db);
			return {
				...after,
				pagesBefore: before.pageCount,
				pagesDirtied: after.pageCount - before.pageCount,
				batches: 1,
				rowsWritten: before.rows,
				elapsedMs: Math.round(performance.now() - startedAt),
				slowestBatchMs,
			};
		},

		// Full correctness gate: SQLite validates its own b-trees, and every row
		// is compared against the payload its id implies.
		//
		// The second half is the part that matters here. `integrity_check` walks
		// structure, so a page served from a stale but structurally valid
		// version passes it; comparing content catches that.
		verify: async (
			c,
			input: { rowBytes?: number; random?: boolean } = {},
		): Promise<
			{
				ok: boolean;
				integrity: string;
				mismatchedRows: number;
				idGaps: number;
			} & StorageStats
		> => {
			const rowBytes = finiteInt(input.rowBytes, DEFAULT_ROW_BYTES);
			const integrityRows = (await c.db.execute(
				"PRAGMA integrity_check",
			)) as Array<Record<string, unknown>>;
			const messages = integrityRows.map((row) =>
				String(Object.values(row)[0]),
			);
			const integrity = messages.join("; ");

			// Random rows cannot be recomputed, so the strongest check left is that
			// every row is the length it was written at. A page served from a
			// stale version still shows up, because a stale page holds different
			// rows entirely and the id gap check below catches that.
			const mismatched = (await c.db.execute(
				input.random
					? `SELECT count(*) AS n FROM big_rows WHERE length(payload) != ${rowBytes}`
					: `SELECT count(*) AS n FROM big_rows WHERE payload != ${payloadExpr("id", rowBytes)}`,
			)) as Array<{ n: number }>;
			// Ids are dense from 1, so max(id) over count(*) catches a row that
			// vanished entirely, which a per-row comparison cannot see.
			const gaps = (await c.db.execute(
				"SELECT coalesce(max(id), 0) - count(*) AS n FROM big_rows",
			)) as Array<{ n: number }>;

			const mismatchedRows = mismatched[0]?.n ?? -1;
			const idGaps = gaps[0]?.n ?? -1;
			return {
				...(await storageStats(c.db)),
				ok:
					messages.length === 1 &&
					messages[0] === "ok" &&
					mismatchedRows === 0 &&
					idGaps === 0,
				integrity,
				mismatchedRows,
				idGaps,
			};
		},

		// Deletes down to `keepRows` and reclaims the freed space, which is the
		// shape truncate cleanup exists for: a commit with a small dirty set and
		// a large drop in database size.
		//
		// VACUUM rewrites the database into its compacted form, so the commit it
		// produces dirties only the pages that survive while the file's EOF falls
		// by everything that did not. The engine has to clear one PIDX row per
		// page above the new EOF inside the commit transaction, and nothing bounds
		// that by the dirty page cap.
		shrink: async (
			c,
			input: { keepRows?: number; deleteBatch?: number } = {},
		): Promise<{
			pagesBefore: number;
			pagesAfter: number;
			pagesDropped: number;
			rowsBefore: number;
			rowsAfter: number;
			deleteMs: number;
			vacuumMs: number;
		}> => {
			const keepRows = finiteInt(input.keepRows, 1_000);
			const deleteBatch = finiteInt(input.deleteBatch, 2_000);
			const before = await storageStats(c.db);

			// Batched so the deletes themselves stay under the single commit cap.
			// The point of the test is the shrink, not a large delete.
			const deleteStartedAt = performance.now();
			for (let id = before.rows; id > keepRows; id -= deleteBatch) {
				const lower = Math.max(keepRows, id - deleteBatch);
				await c.db.execute(
					"DELETE FROM big_rows WHERE id > ? AND id <= ?",
					lower,
					id,
				);
			}
			const deleteMs = Math.round(performance.now() - deleteStartedAt);

			const vacuumStartedAt = performance.now();
			await c.db.execute("VACUUM");
			const vacuumMs = Math.round(performance.now() - vacuumStartedAt);

			const after = await storageStats(c.db);
			return {
				pagesBefore: before.pageCount,
				pagesAfter: after.pageCount,
				pagesDropped: before.pageCount - after.pageCount,
				rowsBefore: before.rows,
				rowsAfter: after.rows,
				deleteMs,
				vacuumMs,
			};
		},

		stats: async (c): Promise<StorageStats> => storageStats(c.db),

		// Content fingerprint that does not depend on page layout, so a database
		// built with one big commit can be compared against one built with many
		// small commits without assuming the two allocate pages identically.
		//
		// Folded to scalars in SQL rather than concatenated, because a digest
		// over a database at the commit cap would itself be about a megabyte on
		// the wire. Sampling one byte per row at an id-dependent offset makes the
		// fold sensitive to content, not just to lengths.
		fingerprint: async (
			c,
		): Promise<{
			rows: number;
			totalBytes: number;
			checksum: number;
			sampleSum: number;
		}> => {
			const row = await queryOne<{
				n: number;
				total: number;
				checksum: number;
				sample_sum: number;
			}>(
				c.db,
				`SELECT count(*) AS n,
				        coalesce(sum(length(payload)), 0) AS total,
				        coalesce(sum((id * 1000003 + length(payload)) % 2147483647), 0) AS checksum,
				        coalesce(sum(unicode(substr(payload, 1 + (id % 64), 1))), 0) AS sample_sum
				 FROM big_rows`,
			);
			return {
				rows: row.n,
				totalBytes: row.total,
				checksum: row.checksum,
				sampleSum: row.sample_sum,
			};
		},
	},
});
