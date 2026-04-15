import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { createClient } from "rivetkit/client";
import { registry } from "../src/index.ts";

const DEFAULT_MB = Number(process.env.BENCH_MB ?? "10");
const DEFAULT_ROWS = Number(process.env.BENCH_ROWS ?? "1");
const DEFAULT_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";

function formatMs(ms: number): string {
	return `${ms.toFixed(1)}ms`;
}

function formatBytes(bytes: number): string {
	const mb = bytes / (1024 * 1024);
	return `${mb.toFixed(2)} MiB`;
}

function runNativeInsert(totalBytes: number, rowCount: number) {
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

async function main() {
	const totalBytes = DEFAULT_MB * 1024 * 1024;
	const rowCount = DEFAULT_ROWS;

	console.log(
		`Benchmarking SQLite insert for ${formatBytes(totalBytes)} across ${rowCount} row(s)`,
	);
	console.log(`Endpoint: ${DEFAULT_ENDPOINT}`);

	registry.start();
	const client = createClient<typeof registry>({ endpoint: DEFAULT_ENDPOINT });
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

	console.log("");
	console.log("RivetKit actor path");
	console.log(`  inserted: ${formatBytes(actorResult.totalBytes)} in ${actorResult.storedRows} row(s)`);
	console.log(`  db insert time: ${formatMs(actorResult.insertElapsedMs)}`);
	console.log(`  db verify time: ${formatMs(actorResult.verifyElapsedMs)}`);
	console.log(`  end-to-end action time: ${formatMs(endToEndElapsedMs)}`);
	console.log(
		`  overhead outside db insert: ${formatMs(endToEndElapsedMs - actorResult.insertElapsedMs)}`,
	);

	console.log("");
	console.log("Native SQLite baseline");
	console.log(`  inserted: ${formatBytes(nativeResult.totalBytes)} in ${nativeResult.storedRows} row(s)`);
	console.log(`  db insert time: ${formatMs(nativeResult.insertElapsedMs)}`);
	console.log(`  db verify time: ${formatMs(nativeResult.verifyElapsedMs)}`);

	console.log("");
	console.log("Delta");
	console.log(
		`  actor db vs native: ${(actorResult.insertElapsedMs / nativeResult.insertElapsedMs).toFixed(2)}x slower`,
	);
	console.log(
		`  end-to-end vs native: ${(endToEndElapsedMs / nativeResult.insertElapsedMs).toFixed(2)}x slower`,
	);

	process.exit(0);
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
