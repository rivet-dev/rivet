import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { createClient } from "rivetkit/client";
import { registry } from "../src/index.ts";

const DEFAULT_MB = Number(process.env.BENCH_MB ?? "10");
const DEFAULT_ROWS = Number(process.env.BENCH_ROWS ?? "1");
const DEFAULT_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const JSON_OUTPUT =
	process.argv.includes("--json") || process.env.BENCH_OUTPUT === "json";

interface BenchmarkInsertResult {
	payloadBytes: number;
	rowCount: number;
	totalBytes: number;
	storedRows: number;
	insertElapsedMs: number;
	verifyElapsedMs: number;
}

interface LargeInsertBenchmarkResult {
	endpoint: string;
	payloadMiB: number;
	totalBytes: number;
	rowCount: number;
	actor: BenchmarkInsertResult;
	native: BenchmarkInsertResult;
	delta: {
		endToEndElapsedMs: number;
		overheadOutsideDbInsertMs: number;
		actorDbVsNativeMultiplier: number;
		endToEndVsNativeMultiplier: number;
	};
}

function formatMs(ms: number): string {
	return `${ms.toFixed(1)}ms`;
}

function formatBytes(bytes: number): string {
	const mb = bytes / (1024 * 1024);
	return `${mb.toFixed(2)} MiB`;
}

function runNativeInsert(
	totalBytes: number,
	rowCount: number,
): BenchmarkInsertResult {
	const dir = mkdtempSync(join(tmpdir(), "sqlite-raw-bench-"));
	const dbPath = join(dir, "bench.db");
	const db = new DatabaseSync(dbPath);

	try {
		db.exec("PRAGMA journal_mode=WAL");
		db.exec("PRAGMA synchronous=NORMAL");
		db.exec(`
			CREATE TABLE payload_bench (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				label TEXT NOT NULL,
				payload TEXT NOT NULL,
				payload_bytes INTEGER NOT NULL,
				created_at INTEGER NOT NULL
			)
		`);

		const payloadBytes = Math.floor(totalBytes / rowCount);
		const payload = "x".repeat(payloadBytes);
		const label = `native-${Date.now()}`;
		const stmt = db.prepare(
			"INSERT INTO payload_bench (label, payload, payload_bytes, created_at) VALUES (?, ?, ?, ?)",
		);
		const start = performance.now();
		db.exec("BEGIN");
		for (let i = 0; i < rowCount; i++) {
			stmt.run(label, payload, payloadBytes, Date.now() + i);
		}
		db.exec("COMMIT");
		const insertElapsedMs = performance.now() - start;

		const verifyStart = performance.now();
		const row = db
			.prepare(
				"SELECT COALESCE(SUM(payload_bytes), 0) as totalBytes, COUNT(*) as storedRows FROM payload_bench WHERE label = ?",
			)
			.get(label) as { totalBytes: number; storedRows: number };
		const verifyElapsedMs = performance.now() - verifyStart;

		return {
			payloadBytes,
			rowCount,
			totalBytes: row.totalBytes,
			storedRows: row.storedRows,
			insertElapsedMs,
			verifyElapsedMs,
		};
	} finally {
		db.close();
		rmSync(dir, { recursive: true, force: true });
	}
}

async function runLargeInsertBenchmark(): Promise<LargeInsertBenchmarkResult> {
	const totalBytes = DEFAULT_MB * 1024 * 1024;
	const rowCount = DEFAULT_ROWS;

	registry.start();
	const client = createClient<typeof registry>({
		endpoint: DEFAULT_ENDPOINT,
	});
	const actor = client.todoList.getOrCreate([`bench-${Date.now()}`]);
	const label = `payload-${crypto.randomUUID()}`;

	const endToEndStart = performance.now();
	const actorResult = await actor.benchInsertPayload(
		label,
		Math.floor(totalBytes / rowCount),
		rowCount,
	);
	const endToEndElapsedMs = performance.now() - endToEndStart;

	const nativeResult = runNativeInsert(totalBytes, rowCount);

	return {
		endpoint: DEFAULT_ENDPOINT,
		payloadMiB: DEFAULT_MB,
		totalBytes,
		rowCount,
		actor: actorResult,
		native: nativeResult,
		delta: {
			endToEndElapsedMs,
			overheadOutsideDbInsertMs:
				endToEndElapsedMs - actorResult.insertElapsedMs,
			actorDbVsNativeMultiplier:
				actorResult.insertElapsedMs / nativeResult.insertElapsedMs,
			endToEndVsNativeMultiplier:
				endToEndElapsedMs / nativeResult.insertElapsedMs,
		},
	};
}

async function main() {
	const result = await runLargeInsertBenchmark();

	if (JSON_OUTPUT) {
		console.log(JSON.stringify(result, null, "\t"));
		process.exit(0);
	}

	console.log(
		`Benchmarking SQLite insert for ${formatBytes(result.totalBytes)} across ${result.rowCount} row(s)`,
	);
	console.log(`Endpoint: ${result.endpoint}`);

	console.log("");
	console.log("RivetKit actor path");
	console.log(
		`  inserted: ${formatBytes(result.actor.totalBytes)} in ${result.actor.storedRows} row(s)`,
	);
	console.log(`  db insert time: ${formatMs(result.actor.insertElapsedMs)}`);
	console.log(`  db verify time: ${formatMs(result.actor.verifyElapsedMs)}`);
	console.log(
		`  end-to-end action time: ${formatMs(result.delta.endToEndElapsedMs)}`,
	);
	console.log(
		`  overhead outside db insert: ${formatMs(result.delta.overheadOutsideDbInsertMs)}`,
	);

	console.log("");
	console.log("Native SQLite baseline");
	console.log(
		`  inserted: ${formatBytes(result.native.totalBytes)} in ${result.native.storedRows} row(s)`,
	);
	console.log(`  db insert time: ${formatMs(result.native.insertElapsedMs)}`);
	console.log(`  db verify time: ${formatMs(result.native.verifyElapsedMs)}`);

	console.log("");
	console.log("Delta");
	console.log(
		`  actor db vs native: ${result.delta.actorDbVsNativeMultiplier.toFixed(2)}x slower`,
	);
	console.log(
		`  end-to-end vs native: ${result.delta.endToEndVsNativeMultiplier.toFixed(2)}x slower`,
	);

	process.exit(0);
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
