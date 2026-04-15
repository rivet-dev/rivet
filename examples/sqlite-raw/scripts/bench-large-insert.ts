import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { ActorError, createClient, type Client } from "rivetkit/client";
import type { SqliteVfsTelemetry } from "rivetkit/db";
import { registry } from "../src/registry.ts";

const DEFAULT_MB = Number(process.env.BENCH_MB ?? "10");
const DEFAULT_ROWS = Number(process.env.BENCH_ROWS ?? "1");
const DEFAULT_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const DEFAULT_STARTUP_GRACE_MS = Number(
	process.env.BENCH_STARTUP_GRACE_MS ?? "5000",
);
const DEFAULT_READY_TIMEOUT_MS = Number(
	process.env.BENCH_READY_TIMEOUT_MS ?? "120000",
);
const DEFAULT_READY_RETRY_MS = Number(
	process.env.BENCH_READY_RETRY_MS ?? "500",
);
const DEFAULT_METRICS_TIMEOUT_MS = Number(
	process.env.BENCH_METRICS_TIMEOUT_MS ?? "1000",
);
const DEFAULT_METRICS_ATTEMPTS = Number(
	process.env.BENCH_METRICS_ATTEMPTS ?? "3",
);
const DEFAULT_METRICS_ENDPOINT =
	process.env.RIVET_METRICS_ENDPOINT ??
	deriveMetricsEndpoint(DEFAULT_ENDPOINT);
const REQUIRE_SERVER_TELEMETRY =
	process.env.BENCH_REQUIRE_SERVER_TELEMETRY === "1";
const JSON_OUTPUT =
	process.argv.includes("--json") || process.env.BENCH_OUTPUT === "json";
const DEBUG_OUTPUT = process.env.BENCH_DEBUG === "1";

type RegistryClient = Client<typeof registry>;

interface BenchmarkInsertResult {
	payloadBytes: number;
	rowCount: number;
	totalBytes: number;
	storedRows: number;
	insertElapsedMs: number;
	verifyElapsedMs: number;
}

interface ActorBenchmarkInsertResult extends BenchmarkInsertResult {
	vfsTelemetry: SqliteVfsTelemetry;
}

interface SqliteServerOperationTelemetry {
	requestCount: number;
	pageEntryCount: number;
	metadataEntryCount: number;
	requestBytes: number;
	payloadBytes: number;
	responseBytes: number;
	durationUs: number;
}

interface SqliteServerWriteValidationTelemetry {
	ok: number;
	lengthMismatch: number;
	tooManyEntries: number;
	payloadTooLarge: number;
	storageQuotaExceeded: number;
	keyTooLarge: number;
	valueTooLarge: number;
}

interface SqliteServerWriteTelemetry extends SqliteServerOperationTelemetry {
	dirtyPageCount: number;
	estimateKvSizeDurationUs: number;
	clearAndRewriteDurationUs: number;
	clearSubspaceCount: number;
	validation: SqliteServerWriteValidationTelemetry;
}

interface SqliteServerTelemetry {
	metricsEndpoint: string;
	path: "generic";
	reads: SqliteServerOperationTelemetry;
	writes: SqliteServerWriteTelemetry;
	truncates: SqliteServerOperationTelemetry;
}

interface LargeInsertBenchmarkResult {
	endpoint: string;
	metricsEndpoint: string;
	payloadMiB: number;
	totalBytes: number;
	rowCount: number;
	actor: ActorBenchmarkInsertResult;
	native: BenchmarkInsertResult;
	serverTelemetry?: SqliteServerTelemetry;
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

type MetricsSnapshot = Map<string, number>;

const SQLITE_METRIC_NAMES = new Set([
	"actor_kv_sqlite_storage_request_total",
	"actor_kv_sqlite_storage_entry_total",
	"actor_kv_sqlite_storage_bytes_total",
	"actor_kv_sqlite_storage_duration_seconds_total",
	"actor_kv_sqlite_storage_phase_duration_seconds_total",
	"actor_kv_sqlite_storage_clear_subspace_total",
	"actor_kv_sqlite_storage_validation_total",
]);

function deriveMetricsEndpoint(endpoint: string): string {
	const url = new URL(endpoint.endsWith("/") ? endpoint : `${endpoint}/`);
	url.port = process.env.RIVET_METRICS_PORT ?? "6430";
	url.pathname = "/metrics";
	url.search = "";
	url.hash = "";
	return url.toString();
}

function metricKey(name: string, labels: Record<string, string>): string {
	const serializedLabels = Object.entries(labels)
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([key, value]) => `${key}=${value}`)
		.join(",");
	return `${name}|${serializedLabels}`;
}

function parseMetricLabels(raw: string): Record<string, string> {
	if (!raw) {
		return {};
	}

	const labels: Record<string, string> = {};
	for (const pair of raw.split(",")) {
		if (!pair) {
			continue;
		}

		const [key, value] = pair.split("=");
		if (!key || value === undefined) {
			continue;
		}

		labels[key.trim()] = value.trim().replace(/^"|"$/g, "");
	}
	return labels;
}

function parsePrometheusMetrics(text: string): MetricsSnapshot {
	const snapshot: MetricsSnapshot = new Map();

	for (const line of text.split("\n")) {
		const trimmed = line.trim();
		if (!trimmed || trimmed.startsWith("#")) {
			continue;
		}

		const match =
			/^([a-zA-Z_:][a-zA-Z0-9_:]*)(?:\{([^}]*)\})?\s+([^\s]+)(?:\s+.*)?$/.exec(
				trimmed,
			);
		if (!match) {
			continue;
		}

		const [, name, rawLabels = "", rawValue] = match;
		if (!SQLITE_METRIC_NAMES.has(name)) {
			continue;
		}

		const value = Number(rawValue);
		if (!Number.isFinite(value)) {
			continue;
		}

		snapshot.set(metricKey(name, parseMetricLabels(rawLabels)), value);
	}

	return snapshot;
}

async function fetchMetricsSnapshot(
	metricsEndpoint: string,
): Promise<MetricsSnapshot | undefined> {
	let lastError: unknown;
	for (let attempt = 0; attempt < DEFAULT_METRICS_ATTEMPTS; attempt += 1) {
		try {
			const response = await fetch(metricsEndpoint, {
				signal: AbortSignal.timeout(DEFAULT_METRICS_TIMEOUT_MS),
			});
			if (!response.ok) {
				throw new Error(
					`Metrics endpoint ${metricsEndpoint} returned ${response.status}.`,
				);
			}

			return parsePrometheusMetrics(await response.text());
		} catch (error) {
			lastError = error;
			await new Promise((resolve) => setTimeout(resolve, 100));
		}
	}

	if (REQUIRE_SERVER_TELEMETRY) {
		throw new Error(
			`Failed to fetch metrics from ${metricsEndpoint}: ${String(lastError)}`,
		);
	}

	debug("metrics scrape unavailable; continuing without server telemetry", {
		metricsEndpoint,
		error:
			lastError instanceof Error
				? { name: lastError.name, message: lastError.message }
				: lastError,
	});

	return undefined;
}

function metricDelta(
	before: MetricsSnapshot,
	after: MetricsSnapshot,
	name: string,
	labels: Record<string, string>,
): number {
	const key = metricKey(name, labels);
	return Math.max(0, (after.get(key) ?? 0) - (before.get(key) ?? 0));
}

function secondsToUs(seconds: number): number {
	return Math.round(seconds * 1_000_000);
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function debug(message: string, ...args: unknown[]): void {
	if (!DEBUG_OUTPUT) {
		return;
	}

	console.error(`[bench-large-insert] ${message}`, ...args);
}

function isRetryableReadinessError(error: unknown): boolean {
	if (error instanceof ActorError) {
		return (
			(error.group === "guard" &&
				(error.code === "actor_ready_timeout" ||
					error.code === "actor_runner_failed")) ||
			(error.group === "core" && error.code === "internal_error")
		);
	}

	if (!(error instanceof Error)) {
		return false;
	}

	return (
		error.message.includes("fetch failed") ||
		error.message.includes("Request timed out") ||
		error.message.includes("pegboard_actor_create timed out") ||
		error.message.includes("Internal Server Error")
	);
}

async function waitForActorRuntimeReady(client: RegistryClient): Promise<void> {
	const deadline = Date.now() + DEFAULT_READY_TIMEOUT_MS;
	let lastError: unknown;
	let attempt = 0;

	while (Date.now() < deadline) {
		try {
			attempt += 1;
			debug("warmup attempt starting", {
				attempt,
				deadline: new Date(deadline).toISOString(),
			});
			const warmupActor = await client.todoList.create([
				`bench-ready-${crypto.randomUUID()}`,
			]);
			debug("warmup actor created", { attempt });
			await warmupActor.addTodo("benchmark-runtime-ready");
			debug("warmup action completed", { attempt });
			return;
		} catch (error) {
			lastError = error;
			debug("warmup attempt failed", {
				attempt,
				error:
					error instanceof Error
						? {
								name: error.name,
								message: error.message,
							}
						: error,
			});
			if (!isRetryableReadinessError(error)) {
				throw error;
			}
			await sleep(DEFAULT_READY_RETRY_MS);
		}
	}

	throw new Error(
		`Timed out waiting ${DEFAULT_READY_TIMEOUT_MS}ms for benchmark actor readiness.`,
		{
			cause: lastError instanceof Error ? lastError : undefined,
		},
	);
}

function buildOperationTelemetry(
	before: MetricsSnapshot,
	after: MetricsSnapshot,
	op: "read" | "write" | "truncate",
): SqliteServerOperationTelemetry {
	return {
		requestCount: metricDelta(before, after, "actor_kv_sqlite_storage_request_total", {
			path: "generic",
			op,
		}),
		pageEntryCount: metricDelta(before, after, "actor_kv_sqlite_storage_entry_total", {
			path: "generic",
			op,
			entry_kind: "page",
		}),
		metadataEntryCount: metricDelta(
			before,
			after,
			"actor_kv_sqlite_storage_entry_total",
			{
				path: "generic",
				op,
				entry_kind: "metadata",
			},
		),
		requestBytes: metricDelta(before, after, "actor_kv_sqlite_storage_bytes_total", {
			path: "generic",
			op,
			byte_kind: "request",
		}),
		payloadBytes: metricDelta(before, after, "actor_kv_sqlite_storage_bytes_total", {
			path: "generic",
			op,
			byte_kind: "payload",
		}),
		responseBytes: metricDelta(before, after, "actor_kv_sqlite_storage_bytes_total", {
			path: "generic",
			op,
			byte_kind: "response",
		}),
		durationUs: secondsToUs(
			metricDelta(
				before,
				after,
				"actor_kv_sqlite_storage_duration_seconds_total",
				{
					path: "generic",
					op,
				},
			),
		),
	};
}

function buildServerTelemetry(
	before: MetricsSnapshot,
	after: MetricsSnapshot,
	metricsEndpoint: string,
): SqliteServerTelemetry {
	const writes = buildOperationTelemetry(before, after, "write");

	return {
		metricsEndpoint,
		path: "generic",
		reads: buildOperationTelemetry(before, after, "read"),
		writes: {
			...writes,
			dirtyPageCount: writes.pageEntryCount,
			estimateKvSizeDurationUs: secondsToUs(
				metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_phase_duration_seconds_total",
					{
						path: "generic",
						phase: "estimate_kv_size",
					},
				),
			),
			clearAndRewriteDurationUs: secondsToUs(
				metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_phase_duration_seconds_total",
					{
						path: "generic",
						phase: "clear_and_rewrite",
					},
				),
			),
			clearSubspaceCount: metricDelta(
				before,
				after,
				"actor_kv_sqlite_storage_clear_subspace_total",
				{
					path: "generic",
				},
			),
			validation: {
				ok: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "ok",
					},
				),
				lengthMismatch: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "length_mismatch",
					},
				),
				tooManyEntries: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "too_many_entries",
					},
				),
				payloadTooLarge: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "payload_too_large",
					},
				),
				storageQuotaExceeded: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "storage_quota_exceeded",
					},
				),
				keyTooLarge: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "key_too_large",
					},
				),
				valueTooLarge: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path: "generic",
						result: "value_too_large",
					},
				),
			},
		},
		truncates: buildOperationTelemetry(before, after, "truncate"),
	};
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

	registry.config.noWelcome = true;
	registry.config.logging = {
		...registry.config.logging,
		level: DEBUG_OUTPUT ? "debug" : "error",
	};
	debug("starting registry");
	registry.start();
	debug("waiting for startup grace", { ms: DEFAULT_STARTUP_GRACE_MS });
	await sleep(DEFAULT_STARTUP_GRACE_MS);

	const client = createClient<typeof registry>({
		endpoint: DEFAULT_ENDPOINT,
	});
	debug("waiting for actor runtime readiness");
	await waitForActorRuntimeReady(client);
	debug("actor runtime ready");
	const actor = client.todoList.getOrCreate([`bench-${Date.now()}`]);
	const label = `payload-${crypto.randomUUID()}`;
	debug("fetching metrics before benchmark");
	const metricsBefore = await fetchMetricsSnapshot(DEFAULT_METRICS_ENDPOINT);

	const endToEndStart = performance.now();
	debug("running measured benchmark action", { label, rowCount, totalBytes });
	const actorResult = await actor.benchInsertPayload(
		label,
		Math.floor(totalBytes / rowCount),
		rowCount,
	);
	const endToEndElapsedMs = performance.now() - endToEndStart;
	debug("fetching metrics after benchmark");
	const metricsAfter = await fetchMetricsSnapshot(DEFAULT_METRICS_ENDPOINT);

	debug("running native insert comparison");
	const nativeResult = runNativeInsert(totalBytes, rowCount);

	return {
		endpoint: DEFAULT_ENDPOINT,
		metricsEndpoint: DEFAULT_METRICS_ENDPOINT,
		payloadMiB: DEFAULT_MB,
		totalBytes,
		rowCount,
		actor: actorResult,
		native: nativeResult,
		serverTelemetry:
			metricsBefore && metricsAfter
				? buildServerTelemetry(
						metricsBefore,
						metricsAfter,
						DEFAULT_METRICS_ENDPOINT,
					)
				: undefined,
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
	console.log(`Metrics endpoint: ${result.metricsEndpoint}`);

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
	console.log(
		result.serverTelemetry
			? `  server write requests: ${result.serverTelemetry.writes.requestCount}, dirty pages: ${result.serverTelemetry.writes.dirtyPageCount}, request bytes: ${formatBytes(result.serverTelemetry.writes.requestBytes)}`
			: "  server telemetry: unavailable",
	);
	console.log(
		result.serverTelemetry
			? `  server estimate_kv_size: ${formatMs(result.serverTelemetry.writes.estimateKvSizeDurationUs / 1000)}, clear-and-rewrite: ${formatMs(result.serverTelemetry.writes.clearAndRewriteDurationUs / 1000)}`
			: "  server estimate_kv_size: unavailable, clear-and-rewrite: unavailable",
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
