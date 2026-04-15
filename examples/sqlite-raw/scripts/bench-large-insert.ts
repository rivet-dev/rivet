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
const DEFAULT_READY_ATTEMPT_TIMEOUT_MS = Number(
	process.env.BENCH_READY_ATTEMPT_TIMEOUT_MS ?? "60000",
);
const DEFAULT_REMOTE_PROBE_TIMEOUT_MS = Number(
	process.env.BENCH_REMOTE_PROBE_TIMEOUT_MS ??
		process.env.BENCH_READY_ATTEMPT_TIMEOUT_MS ??
		"60000",
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
const BENCH_RUNNER_MODE = parseRunnerMode(
	process.env.BENCH_RUNNER_MODE ?? "inline",
);
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
	path: "generic" | "fast_path";
	reads: SqliteServerOperationTelemetry;
	writes: SqliteServerWriteTelemetry;
	truncates: SqliteServerOperationTelemetry;
}

interface LargeInsertBenchmarkResult {
	endpoint: string;
	metricsEndpoint: string;
	runnerMode: "inline" | "remote";
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

function parseRunnerMode(value: string): "inline" | "remote" {
	if (value === "inline" || value === "remote") {
		return value;
	}

	throw new Error(
		`Unsupported BENCH_RUNNER_MODE "${value}". Expected "inline" or "remote".`,
	);
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

function normalizeMetricName(name: string): string {
	return name.startsWith("rivet_") ? name.slice("rivet_".length) : name;
}

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

		const [, rawName, rawLabels = "", rawValue] = match;
		const name = normalizeMetricName(rawName);
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
					error.code === "actor_runner_failed" ||
					error.code === "request_timeout")) ||
			(error.group === "rivetkit" &&
				error.code === "internal_error" &&
				error.message.includes("TimeoutError")) ||
			(error.group === "core" && error.code === "internal_error")
		);
	}

	if (!(error instanceof Error)) {
		return false;
	}

	return (
		error.name === "AbortError" ||
		error.name === "TimeoutError" ||
		error.message.includes("fetch failed") ||
		error.message.includes("Request timed out") ||
		error.message.includes("pegboard_actor_create timed out") ||
		error.message.includes("Internal Server Error")
	);
}

async function waitForActorRuntimeReady(client: RegistryClient): Promise<void> {
	const deadline = Date.now() + DEFAULT_READY_TIMEOUT_MS;
	const readinessKey = [`bench-ready-${crypto.randomUUID()}`];
	const warmupActor = client.todoList.getOrCreate(readinessKey);
	let lastError: unknown;
	let attempt = 0;

	while (Date.now() < deadline) {
		try {
			attempt += 1;
			debug("warmup attempt starting", {
				attempt,
				readinessKey,
				deadline: new Date(deadline).toISOString(),
			});
			debug("warmup actor handle ready", { attempt, readinessKey });
			const actionSignal = AbortSignal.timeout(
				DEFAULT_READY_ATTEMPT_TIMEOUT_MS,
			);
			await warmupActor.action({
				name: "addTodo",
				args: ["benchmark-runtime-ready"],
				signal: actionSignal,
			});
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
	path: "generic" | "fast_path",
	op: "read" | "write" | "truncate",
): SqliteServerOperationTelemetry {
	return {
		requestCount: metricDelta(before, after, "actor_kv_sqlite_storage_request_total", {
			path,
			op,
		}),
		pageEntryCount: metricDelta(before, after, "actor_kv_sqlite_storage_entry_total", {
			path,
			op,
			entry_kind: "page",
		}),
		metadataEntryCount: metricDelta(
			before,
			after,
			"actor_kv_sqlite_storage_entry_total",
			{
				path,
				op,
				entry_kind: "metadata",
			},
		),
		requestBytes: metricDelta(before, after, "actor_kv_sqlite_storage_bytes_total", {
			path,
			op,
			byte_kind: "request",
		}),
		payloadBytes: metricDelta(before, after, "actor_kv_sqlite_storage_bytes_total", {
			path,
			op,
			byte_kind: "payload",
		}),
		responseBytes: metricDelta(before, after, "actor_kv_sqlite_storage_bytes_total", {
			path,
			op,
			byte_kind: "response",
		}),
		durationUs: secondsToUs(
			metricDelta(
				before,
				after,
				"actor_kv_sqlite_storage_duration_seconds_total",
				{
					path,
					op,
				},
			),
		),
	};
}

function selectServerPath(
	before: MetricsSnapshot,
	after: MetricsSnapshot,
): "generic" | "fast_path" {
	const fastPathWrites = metricDelta(
		before,
		after,
		"actor_kv_sqlite_storage_request_total",
		{
			path: "fast_path",
			op: "write",
		},
	);
	const fastPathTruncates = metricDelta(
		before,
		after,
		"actor_kv_sqlite_storage_request_total",
		{
			path: "fast_path",
			op: "truncate",
		},
	);
	if (fastPathWrites > 0 || fastPathTruncates > 0) {
		return "fast_path";
	}
	return "generic";
}

function buildServerTelemetry(
	before: MetricsSnapshot,
	after: MetricsSnapshot,
	metricsEndpoint: string,
): SqliteServerTelemetry {
	const path = selectServerPath(before, after);
	const writes = buildOperationTelemetry(before, after, path, "write");

	return {
		metricsEndpoint,
		path,
		reads: buildOperationTelemetry(before, after, path, "read"),
		writes: {
			...writes,
			dirtyPageCount: writes.pageEntryCount,
			estimateKvSizeDurationUs: secondsToUs(
				metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_phase_duration_seconds_total",
					{
						path,
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
						path,
						phase: "clear_and_rewrite",
					},
				),
			),
			clearSubspaceCount: metricDelta(
				before,
				after,
				"actor_kv_sqlite_storage_clear_subspace_total",
				{
					path,
				},
			),
			validation: {
				ok: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "ok",
					},
				),
				lengthMismatch: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "length_mismatch",
					},
				),
				tooManyEntries: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "too_many_entries",
					},
				),
				payloadTooLarge: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "payload_too_large",
					},
				),
				storageQuotaExceeded: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "storage_quota_exceeded",
					},
				),
				keyTooLarge: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "key_too_large",
					},
				),
				valueTooLarge: metricDelta(
					before,
					after,
					"actor_kv_sqlite_storage_validation_total",
					{
						path,
						result: "value_too_large",
					},
				),
			},
		},
		truncates: buildOperationTelemetry(before, after, path, "truncate"),
	};
}

function assertRemoteServerTelemetry(
	telemetry: SqliteServerTelemetry | undefined,
): SqliteServerTelemetry {
	if (!telemetry) {
		throw new Error(
			"Remote benchmark mode requires server telemetry, but no metrics delta was captured.",
		);
	}

	if (telemetry.writes.requestCount <= 0) {
		throw new Error(
			"Remote benchmark mode expected non-zero server write telemetry, but the write request count stayed at zero.",
		);
	}

	return telemetry;
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

	if (BENCH_RUNNER_MODE === "inline") {
		registry.config.noWelcome = true;
		registry.config.logging = {
			...registry.config.logging,
			level: DEBUG_OUTPUT ? "debug" : "error",
		};
		debug("starting inline registry");
		registry.start();
		debug("waiting for startup grace", { ms: DEFAULT_STARTUP_GRACE_MS });
		await sleep(DEFAULT_STARTUP_GRACE_MS);
	} else {
		debug("skipping inline registry start for remote runner mode");
	}

	const client = createClient<typeof registry>({
		endpoint: DEFAULT_ENDPOINT,
		disableMetadataLookup: true,
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
	if (BENCH_RUNNER_MODE === "remote") {
		debug("running remote storage probe", { label });
		await actor.action({
			name: "benchExerciseStorage",
			args: [label],
			signal: AbortSignal.timeout(DEFAULT_REMOTE_PROBE_TIMEOUT_MS),
		});
	}
	debug("fetching metrics after benchmark");
	const metricsAfter = await fetchMetricsSnapshot(DEFAULT_METRICS_ENDPOINT);
	const serverTelemetry =
		metricsBefore && metricsAfter
			? buildServerTelemetry(
					metricsBefore,
					metricsAfter,
					DEFAULT_METRICS_ENDPOINT,
				)
			: undefined;
	const resolvedServerTelemetry =
		BENCH_RUNNER_MODE === "remote"
			? assertRemoteServerTelemetry(serverTelemetry)
			: serverTelemetry;

	debug("running native insert comparison");
	const nativeResult = runNativeInsert(totalBytes, rowCount);

	return {
		endpoint: DEFAULT_ENDPOINT,
		metricsEndpoint: DEFAULT_METRICS_ENDPOINT,
		runnerMode: BENCH_RUNNER_MODE,
		payloadMiB: DEFAULT_MB,
		totalBytes,
		rowCount,
		actor: actorResult,
		native: nativeResult,
		serverTelemetry: resolvedServerTelemetry,
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
	console.log(`Runner mode: ${result.runnerMode}`);

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
