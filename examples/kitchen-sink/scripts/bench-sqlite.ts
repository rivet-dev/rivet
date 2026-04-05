#!/usr/bin/env -S npx tsx

/**
 * SQLite Benchmark Runner
 *
 * Spins up actors against the kitchen-sink registry and runs each benchmark
 * scenario, printing a summary table at the end.
 *
 * Usage:
 *   # Against local engine (spawn_engine=true default):
 *   npx tsx scripts/bench-sqlite.ts
 *
 *   # Against a remote endpoint:
 *   RIVET_ENDPOINT=http://localhost:6420 npx tsx scripts/bench-sqlite.ts
 *
 *   # Quick mode (smaller datasets):
 *   npx tsx scripts/bench-sqlite.ts --quick
 */

import { setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { sqliteBench } from "../src/actors/sqlite-bench.ts";

const registry = setup({ use: { sqliteBench } });
type Registry = typeof registry;

// ── Config ──────────────────────────────────────────────────────────

const QUICK = process.argv.includes("--quick");

const SIZES = QUICK
	? { small: 100, medium: 500, large: 1000, growth: 2000, growthInterval: 500 }
	: { small: 500, medium: 2000, large: 10000, growth: 10000, growthInterval: 2000 };

// Orders of magnitude for scale sweep benchmarks.
const SCALES = QUICK
	? [1, 10, 100, 1000, 10000]
	: [1, 10, 100, 1000, 10000, 100000];

// Per-operation timeout for large scale tests (ms).
const SCALE_TIMEOUT_MS = 120_000;

// Output file for markdown report.
const REPORT_FILE = process.env.BENCH_REPORT;

// ── Types ───────────────────────────────────────────────────────────

interface BenchmarkEntry {
	name: string;
	elapsedMs: number;
	detail?: string;
}

// ── Helpers ─────────────────────────────────────────────────────────

function ms(n: number): string {
	return `${n.toFixed(1)}ms`;
}

function perOp(total: number, count: number): string {
	return `${(total / count).toFixed(3)}ms/op`;
}

/** Run a benchmark with a timeout. Returns null if it times out. */
async function withTimeout<T>(fn: () => Promise<T>, timeoutMs: number): Promise<T | null> {
	return Promise.race([
		fn(),
		new Promise<null>((resolve) => setTimeout(() => resolve(null), timeoutMs)),
	]);
}

function printTable(entries: BenchmarkEntry[]): void {
	const nameWidth = Math.max(40, ...entries.map((e) => e.name.length));
	const timeWidth = 14;
	const detailWidth = 40;

	const sep = "-".repeat(nameWidth + timeWidth + detailWidth + 8);
	console.log(sep);
	console.log(
		`${"Benchmark".padEnd(nameWidth)}  ${"Time".padStart(timeWidth)}  ${"Detail".padEnd(detailWidth)}`,
	);
	console.log(sep);
	for (const e of entries) {
		console.log(
			`${e.name.padEnd(nameWidth)}  ${ms(e.elapsedMs).padStart(timeWidth)}  ${(e.detail ?? "").padEnd(detailWidth)}`,
		);
	}
	console.log(sep);
}

// ── Runner ──────────────────────────────────────────────────────────

type Client = Awaited<ReturnType<typeof registry.start>>["client"];

async function freshActor(client: Client) {
	return client.sqliteBench.getOrCreate([`bench-${crypto.randomUUID()}`]);
}

async function runAll(client: Client): Promise<BenchmarkEntry[]> {
	const entries: BenchmarkEntry[] = [];

	// 1. Large migrations
	console.log("  [1/14] Large migrations...");
	{
		const a = await freshActor(client);
		const r = await a.benchMigration(QUICK ? 50 : 100);
		entries.push({
			name: `Migration (${r.tableCount} tables + indexes)`,
			elapsedMs: r.elapsedMs,
			detail: perOp(r.elapsedMs, r.tableCount),
		});
	}

	// 1b. Large migrations in transaction
	console.log("  [1b/14] Large migrations (transaction)...");
	{
		const a = await freshActor(client);
		const r = await a.benchMigrationTransaction(QUICK ? 50 : 100);
		entries.push({
			name: `Migration TX (${r.tableCount} tables + indexes)`,
			elapsedMs: r.elapsedMs,
			detail: perOp(r.elapsedMs, r.tableCount),
		});
	}

	// 2. Single-row inserts (scale sweep)
	console.log("  [2] Single-row inserts (scale sweep)...");
	for (const n of SCALES) {
		process.stdout.write(`    x${n}...`);
		const a = await freshActor(client);
		const r = await withTimeout(() => a.benchInsertSingle(n), SCALE_TIMEOUT_MS);
		if (r) {
			console.log(` ${ms(r.elapsedMs)}`);
			entries.push({ name: `Insert single x${n}`, elapsedMs: r.elapsedMs, detail: perOp(r.elapsedMs, n) });
		} else {
			console.log(" TIMEOUT");
			entries.push({ name: `Insert single x${n}`, elapsedMs: -1, detail: "TIMEOUT" });
			break;
		}
	}

	// 3. Batch inserts (scale sweep)
	console.log("  [3] Batch inserts (scale sweep)...");
	for (const n of SCALES) {
		process.stdout.write(`    x${n}...`);
		const a = await freshActor(client);
		const r = await withTimeout(() => a.benchInsertBatch(n, 50), SCALE_TIMEOUT_MS);
		if (r) {
			console.log(` ${ms(r.elapsedMs)}`);
			entries.push({ name: `Insert batch x${n}`, elapsedMs: r.elapsedMs, detail: perOp(r.elapsedMs, n) });
		} else {
			console.log(" TIMEOUT");
			entries.push({ name: `Insert batch x${n}`, elapsedMs: -1, detail: "TIMEOUT" });
			break;
		}
	}

	// 4. Transactional inserts (scale sweep)
	console.log("  [4] TX inserts (scale sweep)...");
	for (const n of SCALES) {
		process.stdout.write(`    x${n}...`);
		const a = await freshActor(client);
		const r = await withTimeout(() => a.benchInsertTransaction(n), SCALE_TIMEOUT_MS);
		if (r) {
			console.log(` ${ms(r.elapsedMs)}`);
			entries.push({ name: `Insert TX x${n}`, elapsedMs: r.elapsedMs, detail: perOp(r.elapsedMs, n) });
		} else {
			console.log(" TIMEOUT");
			entries.push({ name: `Insert TX x${n}`, elapsedMs: -1, detail: "TIMEOUT" });
			break;
		}
	}

	// 5. Point reads (scale sweep)
	console.log("  [5] Point reads (scale sweep)...");
	for (const n of SCALES) {
		process.stdout.write(`    x${n}...`);
		const a = await freshActor(client);
		const r = await withTimeout(() => a.benchPointRead(n), SCALE_TIMEOUT_MS);
		if (r) {
			console.log(` ${ms(r.elapsedMs)}`);
			entries.push({ name: `Point read x${n}`, elapsedMs: r.elapsedMs, detail: perOp(r.elapsedMs, n) });
		} else {
			console.log(" TIMEOUT");
			entries.push({ name: `Point read x${n}`, elapsedMs: -1, detail: "TIMEOUT" });
			break;
		}
	}

	// 6. Full table scan
	console.log("  [6/14] Full table scan...");
	{
		const a = await freshActor(client);
		const r = await a.benchFullScan(SIZES.medium);
		entries.push({
			name: `Full scan (${r.rowsReturned} rows)`,
			elapsedMs: r.elapsedMs,
		});
	}

	// 7. Range scan
	console.log("  [7/14] Range scan...");
	{
		const a = await freshActor(client);
		const r = await a.benchRangeScan(SIZES.medium);
		entries.push({
			name: `Range scan indexed (${r.indexed.rowsReturned} rows)`,
			elapsedMs: r.indexed.elapsedMs,
		});
		entries.push({
			name: `Range scan unindexed (${r.unindexed.rowsReturned} rows)`,
			elapsedMs: r.unindexed.elapsedMs,
		});
	}

	// 8. Large payloads
	console.log("  [8/14] Large payloads...");
	{
		const a = await freshActor(client);
		const r = await a.benchLargePayload(100, 4096);
		entries.push({
			name: `Large payload insert (4KB x ${r.rowCount})`,
			elapsedMs: r.insertElapsedMs,
			detail: perOp(r.insertElapsedMs, r.rowCount),
		});
		entries.push({
			name: `Large payload read (4KB x ${r.rowsRead})`,
			elapsedMs: r.readElapsedMs,
		});
	}
	{
		const a = await freshActor(client);
		const r = await a.benchLargePayload(20, 32768);
		entries.push({
			name: `Large payload insert (32KB x ${r.rowCount})`,
			elapsedMs: r.insertElapsedMs,
			detail: perOp(r.insertElapsedMs, r.rowCount),
		});
		entries.push({
			name: `Large payload read (32KB x ${r.rowsRead})`,
			elapsedMs: r.readElapsedMs,
		});
	}

	// 9. Complex queries
	console.log("  [9/14] Complex queries...");
	{
		const a = await freshActor(client);
		const r = await a.benchComplexQueries(SIZES.medium);
		for (const [queryType, result] of Object.entries(r.results)) {
			entries.push({
				name: `Complex: ${queryType} (${result.rowCount} rows)`,
				elapsedMs: result.elapsedMs,
			});
		}
	}

	// 10. Bulk update
	console.log("  [10/14] Bulk update...");
	{
		const a = await freshActor(client);
		const r = await a.benchBulkUpdate(SIZES.medium);
		entries.push({
			name: `Bulk update (~${Math.floor(r.seedRows / 2)} rows)`,
			elapsedMs: r.elapsedMs,
		});
	}

	// 11. Bulk delete + VACUUM
	console.log("  [11/14] Bulk delete + VACUUM...");
	{
		const a = await freshActor(client);
		const r = await a.benchDeleteVacuum(SIZES.medium);
		entries.push({
			name: `Bulk delete (~${Math.floor(r.seedRows / 2)} rows)`,
			elapsedMs: r.deleteElapsedMs,
		});
		entries.push({
			name: `VACUUM after delete`,
			elapsedMs: r.vacuumElapsedMs,
		});
	}

	// 12. Mixed OLTP (scale sweep)
	console.log("  [12] Mixed OLTP (scale sweep)...");
	for (const n of SCALES) {
		process.stdout.write(`    x${n}...`);
		const a = await freshActor(client);
		const r = await withTimeout(() => a.benchMixedOltp(n, 0.7), SCALE_TIMEOUT_MS);
		if (r) {
			console.log(` ${ms(r.elapsedMs)}`);
			entries.push({ name: `Mixed OLTP x${n} (${r.reads}R/${r.writes}W)`, elapsedMs: r.elapsedMs, detail: perOp(r.elapsedMs, n) });
		} else {
			console.log(" TIMEOUT");
			entries.push({ name: `Mixed OLTP x${n}`, elapsedMs: -1, detail: "TIMEOUT" });
			break;
		}
	}

	// 13. Hot row (scale sweep)
	console.log("  [13] Hot row (scale sweep)...");
	for (const n of SCALES) {
		process.stdout.write(`    x${n}...`);
		const a = await freshActor(client);
		const r = await withTimeout(() => a.benchHotRow(n), SCALE_TIMEOUT_MS);
		if (r) {
			console.log(` ${ms(r.elapsedMs)}`);
			entries.push({ name: `Hot row updates x${n}`, elapsedMs: r.elapsedMs, detail: perOp(r.elapsedMs, n) });
		} else {
			console.log(" TIMEOUT");
			entries.push({ name: `Hot row updates x${n}`, elapsedMs: -1, detail: "TIMEOUT" });
			break;
		}
	}

	// 14. JSON operations
	console.log("  [14/14] JSON operations...");
	{
		const a = await freshActor(client);
		const r = await a.benchJson(SIZES.small);
		entries.push({
			name: `JSON insert x${r.rowCount}`,
			elapsedMs: r.insertElapsedMs,
			detail: perOp(r.insertElapsedMs, r.rowCount),
		});
		entries.push({
			name: `JSON extract query (${r.jsonExtract.rowCount} rows)`,
			elapsedMs: r.jsonExtract.elapsedMs,
		});
		entries.push({
			name: `JSON each aggregation (${r.jsonEach.rowCount} groups)`,
			elapsedMs: r.jsonEach.elapsedMs,
		});
	}

	// FTS and growth are optional since FTS5 may not be available in all builds.
	console.log("  [bonus] FTS5...");
	try {
		const a = await freshActor(client);
		const r = await a.benchFts(SIZES.small);
		entries.push({
			name: `FTS5 insert x${r.docCount}`,
			elapsedMs: r.insertElapsedMs,
			detail: perOp(r.insertElapsedMs, r.docCount),
		});
		entries.push({
			name: `FTS5 search (${r.search.rowCount} hits)`,
			elapsedMs: r.search.elapsedMs,
		});
		entries.push({
			name: `FTS5 prefix search (${r.prefixSearch.rowCount} hits)`,
			elapsedMs: r.prefixSearch.elapsedMs,
		});
	} catch (err) {
		console.log(`    Skipped FTS5: ${err}`);
	}

	console.log("  [bonus] Growth test...");
	{
		const a = await freshActor(client);
		const r = await a.benchGrowth(SIZES.growth, SIZES.growthInterval);
		for (const m of r.measurements) {
			entries.push({
				name: `Growth @${m.rowCount} rows: insert batch`,
				elapsedMs: m.insertBatchMs,
				detail: perOp(m.insertBatchMs, SIZES.growthInterval),
			});
			entries.push({
				name: `Growth @${m.rowCount} rows: 100 point reads`,
				elapsedMs: m.pointReadMs,
				detail: perOp(m.pointReadMs, 100),
			});
		}
	}

	return entries;
}

// ── Concurrent actor benchmark ──────────────────────────────────────

const CONCURRENCY_SCALES = [1, 5, 10, 50, 100];
const ROWS_PER_CONCURRENT_ACTOR = 100;

async function runConcurrent(
	client: Client,
): Promise<BenchmarkEntry[]> {
	const entries: BenchmarkEntry[] = [];

	for (const actorCount of CONCURRENCY_SCALES) {
		process.stdout.write(`  Concurrent: ${actorCount} actors x ${ROWS_PER_CONCURRENT_ACTOR} rows...`);
		const start = performance.now();
		const promises = Array.from({ length: actorCount }, async () => {
			const a = await freshActor(client);
			return a.benchInsertTransaction(ROWS_PER_CONCURRENT_ACTOR);
		});
		const results = await withTimeout(
			() => Promise.all(promises),
			SCALE_TIMEOUT_MS,
		);
		if (!results) {
			console.log(" TIMEOUT");
			entries.push({
				name: `Concurrent x${actorCount} wall`,
				elapsedMs: -1,
				detail: "TIMEOUT",
			});
			break;
		}
		const totalMs = performance.now() - start;
		const avgMs =
			results.reduce((sum, r) => sum + r.elapsedMs, 0) / results.length;
		const totalRows = actorCount * ROWS_PER_CONCURRENT_ACTOR;
		console.log(` ${totalMs.toFixed(0)}ms wall, ${avgMs.toFixed(1)}ms avg/actor`);
		entries.push({
			name: `Concurrent x${actorCount} wall`,
			elapsedMs: totalMs,
			detail: `${totalRows} total rows`,
		});
		entries.push({
			name: `Concurrent x${actorCount} avg/actor`,
			elapsedMs: avgMs,
			detail: perOp(avgMs, ROWS_PER_CONCURRENT_ACTOR),
		});
		entries.push({
			name: `Concurrent x${actorCount} throughput`,
			elapsedMs: totalRows / (totalMs / 1000),
			detail: `rows/sec`,
		});
	}

	return entries;
}

// ── Native SQLite baseline (no VFS, no KV channel, raw disk) ───────

async function runNativeBaseline(): Promise<BenchmarkEntry[]> {
	const { DatabaseSync } = await import("node:sqlite");
	const { mkdtempSync, rmSync } = await import("node:fs");
	const { join } = await import("node:path");
	const { tmpdir } = await import("node:os");

	const dir = mkdtempSync(join(tmpdir(), "sqlite-bench-"));
	const dbPath = join(dir, "bench.db");
	const db = new DatabaseSync(dbPath);
	db.exec("PRAGMA journal_mode=WAL");
	db.exec("PRAGMA synchronous=NORMAL");

	// Create the base table (matches actor onMigrate)
	db.exec(`
		CREATE TABLE IF NOT EXISTS bench (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			key TEXT, value TEXT, num REAL, created_at INTEGER NOT NULL
		)
	`);
	db.exec("CREATE INDEX IF NOT EXISTS idx_bench_key ON bench(key)");
	db.exec("CREATE INDEX IF NOT EXISTS idx_bench_num ON bench(num)");

	const entries: BenchmarkEntry[] = [];
	const tableCount = QUICK ? 50 : 100;

	// Migration (no transaction)
	{
		const start = performance.now();
		for (let i = 0; i < tableCount; i++) {
			db.exec(`CREATE TABLE IF NOT EXISTS baseline_t${i} (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				a TEXT, b TEXT, c REAL, d INTEGER, created_at INTEGER NOT NULL
			)`);
			db.exec(`CREATE INDEX IF NOT EXISTS idx_baseline_t${i}_a ON baseline_t${i}(a)`);
		}
		entries.push({
			name: `[baseline] Migration (${tableCount} tables)`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, tableCount),
		});
	}

	// Migration in transaction
	{
		const start = performance.now();
		db.exec("BEGIN");
		for (let i = 0; i < tableCount; i++) {
			db.exec(`CREATE TABLE IF NOT EXISTS baseline_tx_t${i} (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				a TEXT, b TEXT, c REAL, d INTEGER, created_at INTEGER NOT NULL
			)`);
			db.exec(`CREATE INDEX IF NOT EXISTS idx_baseline_tx_t${i}_a ON baseline_tx_t${i}(a)`);
		}
		db.exec("COMMIT");
		entries.push({
			name: `[baseline] Migration TX (${tableCount} tables)`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, tableCount),
		});
	}

	// Single-row inserts
	{
		const count = SIZES.small;
		const stmt = db.prepare("INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)");
		const start = performance.now();
		for (let i = 0; i < count; i++) {
			stmt.run(`key-${i}`, `value-${i}`, Math.random(), Date.now());
		}
		entries.push({
			name: `[baseline] Insert single-row x${count}`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, count),
		});
	}

	// Insert in transaction
	{
		const count = SIZES.small;
		const stmt = db.prepare("INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)");
		const start = performance.now();
		db.exec("BEGIN");
		for (let i = 0; i < count; i++) {
			stmt.run(`tx-key-${i}`, `tx-value-${i}`, Math.random(), Date.now());
		}
		db.exec("COMMIT");
		entries.push({
			name: `[baseline] Insert TX x${count}`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, count),
		});
	}

	// Point reads
	{
		const count = SIZES.small;
		const stmt = db.prepare("SELECT * FROM bench WHERE key = ?");
		const start = performance.now();
		for (let i = 0; i < count; i++) {
			stmt.get(`key-${i}`);
		}
		entries.push({
			name: `[baseline] Point read x${count}`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, count),
		});
	}

	// Full scan
	{
		const start = performance.now();
		const rows = db.prepare("SELECT * FROM bench").all();
		entries.push({
			name: `[baseline] Full scan (${rows.length} rows)`,
			elapsedMs: performance.now() - start,
		});
	}

	// Hot row updates
	{
		db.exec("INSERT INTO bench (key, value, num, created_at) VALUES ('hot', 'row', 0, 0)");
		const count = SIZES.small;
		const stmt = db.prepare("UPDATE bench SET num = ? WHERE key = 'hot'");
		const start = performance.now();
		for (let i = 0; i < count; i++) {
			stmt.run(i);
		}
		entries.push({
			name: `[baseline] Hot row updates x${count}`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, count),
		});
	}

	// Mixed OLTP
	{
		const count = SIZES.small;
		const readStmt = db.prepare("SELECT * FROM bench WHERE key = ?");
		const writeStmt = db.prepare("INSERT INTO bench (key, value, num, created_at) VALUES (?, ?, ?, ?)");
		let reads = 0, writes = 0;
		const start = performance.now();
		for (let i = 0; i < count; i++) {
			if (Math.random() < 0.7) {
				readStmt.get(`key-${Math.floor(Math.random() * count)}`);
				reads++;
			} else {
				writeStmt.run(`oltp-${i}`, `val-${i}`, Math.random(), Date.now());
				writes++;
			}
		}
		entries.push({
			name: `[baseline] Mixed OLTP x${count} (${reads}R/${writes}W)`,
			elapsedMs: performance.now() - start,
			detail: perOp(performance.now() - start, count),
		});
	}

	db.close();
	rmSync(dir, { recursive: true });
	return entries;
}

// ── Main ────────────────────────────────────────────────────────────

async function main(): Promise<void> {
	console.log(`SQLite Benchmark (${QUICK ? "quick" : "full"} mode)\n`);

	const endpoint = process.env.RIVET_ENDPOINT;

	let client: ReturnType<typeof createClient<Registry>>;
	if (endpoint) {
		console.log(`Connecting to endpoint: ${endpoint}\n`);
		registry.start();
		client = createClient<Registry>({ endpoint });
	} else {
		console.log("Starting with local file-system driver\n");
		registry.start();
		client = createClient<Registry>({ endpoint: "http://localhost:6420" });
	}

	// Give runner time to connect to the engine
	if (endpoint) {
		console.log("Waiting for runner to connect...");
		// Poll until the engine has a runner available
		for (let i = 0; i < 30; i++) {
			try {
				const res = await fetch(`${endpoint}/actors?namespace=default`, {
					method: "PUT",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify({ name: "sqliteBench", key: ["__health_check__"] }),
				});
				if (res.ok) {
					console.log("Runner connected!\n");
					break;
				}
				const body = await res.json().catch(() => ({}));
				if ((body as any)?.error?.code !== "no_runners_available") {
					console.log("Runner connected!\n");
					break;
				}
			} catch {}
			await new Promise(r => setTimeout(r, 1000));
			process.stdout.write(".");
		}
		console.log("");
	}

	console.log("Running benchmarks...\n");
	const entries = await runAll(client);

	// Concurrent actor test.
	console.log("\nRunning concurrent actor scale sweep...");
	const concurrentEntries = await runConcurrent(client);
	entries.push(...concurrentEntries);

	// Native SQLite baseline (raw disk, no VFS/KV).
	console.log("\nRunning native SQLite baseline...");
	const baselineEntries = await runNativeBaseline();
	entries.push(...baselineEntries);

	console.log("\n");
	printTable(entries);

	// Summary stats.
	const totalMs = entries.reduce((sum, e) => sum + (e.elapsedMs > 0 ? e.elapsedMs : 0), 0);
	console.log(`\nTotal benchmark time: ${ms(totalMs)}`);
	console.log(`Scenarios run: ${entries.length}`);

	// Write JSON results for report generation.
	const jsonFile = REPORT_FILE ? REPORT_FILE.replace(/\.md$/, ".json") : `/tmp/bench-results-${Date.now()}.json`;
	const { writeFileSync } = await import("node:fs");
	writeFileSync(jsonFile, JSON.stringify(entries, null, 2));
	console.log(`\nResults written to ${jsonFile}`);

	// Print KV channel metrics if native SQLite is available.
	try {
		// Access the internal native-sqlite module to get KV channel metrics.
		// Uses createRequire for CJS compat with the napi addon.
		const { createRequire } = await import("node:module");
		const require = createRequire(import.meta.url);
		const native = require("@rivetkit/sqlite-native");
		// The kvChannel handle is stored as a module-level singleton in native-sqlite.ts.
		// We can't access it directly, but we exported getKvChannelMetrics.
		// For the bench, we'll try the direct path.
		const nativeSqlite = await import(
			// @ts-ignore
			"../../../rivetkit-typescript/packages/rivetkit/src/db/native-sqlite.ts"
		);
		const m = nativeSqlite.getKvChannelMetrics?.();
		if (m) {
			console.log("\n--- KV Channel Metrics ---");
			const ops: [string, any][] = [
				["get", m.get],
				["put", m.put],
				["delete", m.delete],
				["deleteRange", m.deleteRange],
				["actorOpen", m.actorOpen],
				["actorClose", m.actorClose],
			];
			const nameWidth = 14;
			const colWidth = 12;
			console.log(
				`${"Op".padEnd(nameWidth)}  ${"Count".padStart(colWidth)}  ${"Avg (us)".padStart(colWidth)}  ${"Min (us)".padStart(colWidth)}  ${"Max (us)".padStart(colWidth)}  ${"Total (ms)".padStart(colWidth)}`,
			);
			console.log("-".repeat(nameWidth + colWidth * 5 + 10));
			for (const [name, s] of ops) {
				if (s && s.count > 0) {
					console.log(
						`${name.padEnd(nameWidth)}  ${String(s.count).padStart(colWidth)}  ${s.avgDurationUs.toFixed(0).padStart(colWidth)}  ${String(s.minDurationUs).padStart(colWidth)}  ${String(s.maxDurationUs).padStart(colWidth)}  ${(s.totalDurationUs / 1000).toFixed(1).padStart(colWidth)}`,
					);
				}
			}
		}
	} catch {
		// Native module or metrics not available, skip.
	}

	process.exit(0);
}

main().catch((err) => {
	console.error("Benchmark failed:", err);
	process.exit(1);
});
