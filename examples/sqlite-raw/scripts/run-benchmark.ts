import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
	existsSync,
	readdirSync,
	readFileSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const exampleDir = resolve(__dirname, "..");
const repoRoot = resolve(exampleDir, "../..");
const resultsJsonPath = resolve(exampleDir, "bench-results.json");
const resultsMarkdownPath = resolve(exampleDir, "BENCH_RESULTS.md");
const phaseLabels = {
	"phase-0": "Phase 0",
	"phase-1": "Phase 1",
	"phase-2-3": "Phase 2/3",
	final: "Final",
} as const;
const phaseOrder = ["phase-0", "phase-1", "phase-2-3", "final"] as const;
const defaultBatchCeilingPages = [128, 512, 1024, 2048, 3328] as const;
const sqlitePageSizeBytes = 4096;
const sqlitePageOverheadEstimate = 32;
const defaultEndpoint = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const defaultLogPath = "/tmp/sqlite-raw-bench-engine.log";
const defaultFreshEngineReadyTimeoutMs =
	process.env.BENCH_READY_TIMEOUT_MS ?? "300000";
const defaultRustLog =
	"opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug";

type PhaseKey = (typeof phaseOrder)[number];

interface CliOptions {
	phase?: PhaseKey;
	evaluateBatchCeiling: boolean;
	chosenLimitPages?: number;
	batchPages?: number[];
	freshEngine: boolean;
	renderOnly: boolean;
}

interface BenchmarkInsertResult {
	payloadBytes: number;
	rowCount: number;
	totalBytes: number;
	storedRows: number;
	insertElapsedMs: number;
	verifyElapsedMs: number;
}

interface ActorLargeInsertBenchmarkResult extends BenchmarkInsertResult {
	vfsTelemetry: SqliteVfsTelemetry;
}

interface LargeInsertBenchmarkResult {
	endpoint: string;
	metricsEndpoint?: string;
	payloadMiB: number;
	totalBytes: number;
	rowCount: number;
	actor: ActorLargeInsertBenchmarkResult;
	native: BenchmarkInsertResult;
	serverTelemetry?: SqliteServerTelemetry;
	delta: {
		endToEndElapsedMs: number;
		overheadOutsideDbInsertMs: number;
		actorDbVsNativeMultiplier: number;
		endToEndVsNativeMultiplier: number;
	};
}

interface SqliteVfsReadTelemetry {
	count: number;
	durationUs: number;
	requestedBytes: number;
	returnedBytes: number;
	shortReadCount: number;
}

interface SqliteVfsWriteTelemetry {
	count: number;
	durationUs: number;
	inputBytes: number;
	bufferedCount: number;
	bufferedBytes: number;
	immediateKvPutCount: number;
	immediateKvPutBytes: number;
}

interface SqliteVfsSyncTelemetry {
	count: number;
	durationUs: number;
	metadataFlushCount: number;
	metadataFlushBytes: number;
}

interface SqliteVfsAtomicWriteTelemetry {
	beginCount: number;
	commitAttemptCount: number;
	commitSuccessCount: number;
	commitDurationUs: number;
	committedDirtyPagesTotal: number;
	maxCommittedDirtyPages: number;
	committedBufferedBytesTotal: number;
	rollbackCount: number;
	fastPathAttemptCount?: number;
	fastPathSuccessCount?: number;
	fastPathFallbackCount?: number;
	fastPathFailureCount?: number;
	fastPathDirtyPagesTotal?: number;
	maxFastPathDirtyPages?: number;
	fastPathRequestBytesTotal?: number;
	maxFastPathRequestBytes?: number;
	fastPathDurationUs?: number;
	maxFastPathDurationUs?: number;
	batchCapFailureCount: number;
	commitKvPutFailureCount: number;
}

interface SqliteVfsKvTelemetry {
	getCount: number;
	getDurationUs: number;
	getKeyCount: number;
	getBytes: number;
	putCount: number;
	putDurationUs: number;
	putKeyCount: number;
	putBytes: number;
	deleteCount: number;
	deleteDurationUs: number;
	deleteKeyCount: number;
	deleteRangeCount: number;
	deleteRangeDurationUs: number;
}

interface SqliteVfsTelemetry {
	reads: SqliteVfsReadTelemetry;
	writes: SqliteVfsWriteTelemetry;
	syncs: SqliteVfsSyncTelemetry;
	atomicWrite: SqliteVfsAtomicWriteTelemetry;
	kv: SqliteVfsKvTelemetry;
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

interface BuildProvenance {
	command: string;
	cwd: string;
	durationMs: number;
	artifact: string | null;
	artifactModifiedAt: string | null;
}

interface BenchRun {
	id: string;
	phase: PhaseKey;
	recordedAt: string;
	gitSha: string;
	workflowCommand: string;
	benchmarkCommand: string;
	endpoint: string;
	freshEngineStart: boolean;
	engineLogPath: string | null;
	engineBuild: BuildProvenance;
	nativeBuild: BuildProvenance;
	benchmark: LargeInsertBenchmarkResult;
}

interface BatchCeilingSample {
	targetDirtyPages: number;
	payloadMiB: number;
	benchmarkCommand: string;
	benchmark: LargeInsertBenchmarkResult;
}

interface BatchCeilingEvaluation {
	id: string;
	recordedAt: string;
	gitSha: string;
	workflowCommand: string;
	endpoint: string;
	freshEngineStart: boolean;
	engineLogPath: string | null;
	engineBuild: BuildProvenance;
	nativeBuild: BuildProvenance;
	chosenLimitPages: number;
	batchPages: number[];
	notes: string[];
	samples: BatchCeilingSample[];
}

interface BenchResultsStore {
	schemaVersion: 1;
	sourceFile: string;
	resultsFile: string;
	runs: BenchRun[];
	batchCeilingEvaluations?: BatchCeilingEvaluation[];
}

function printUsage(): void {
	console.log(`Usage:
  pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-0 [--fresh-engine]
  pnpm --dir examples/sqlite-raw run bench:record -- --evaluate-batch-ceiling --chosen-limit-pages 3328 [--batch-pages 128,512,1024,2048,3328] [--fresh-engine]
  pnpm --dir examples/sqlite-raw run bench:record -- --render-only

Options:
  --phase <phase-0|phase-1|phase-2-3|final>
  --evaluate-batch-ceiling
  --chosen-limit-pages <pages>
  --batch-pages <comma-separated pages>
  --fresh-engine   Build and start a fresh local engine before the benchmark
  --render-only    Regenerate BENCH_RESULTS.md from bench-results.json

Environment:
  BENCH_MB         Payload size in MiB. Defaults to 10.
  BENCH_ROWS       Number of rows. Defaults to 1.
  RIVET_ENDPOINT   Engine endpoint. Defaults to http://127.0.0.1:6420.
`);
}

function parseNumberList(raw: string): number[] {
	const values = raw
		.split(",")
		.map((value) => Number(value.trim()))
		.filter((value) => Number.isFinite(value) && value > 0);
	if (values.length === 0) {
		throw new Error(`Expected a comma-separated list of positive numbers, got "${raw}".`);
	}
	return [...new Set(values)].sort((a, b) => a - b);
}

function parseArgs(argv: string[]): CliOptions {
	const options: CliOptions = {
		evaluateBatchCeiling: false,
		freshEngine: false,
		renderOnly: false,
	};

	for (let i = 0; i < argv.length; i += 1) {
		const arg = argv[i];
		if (arg === "--") {
			continue;
		}
		if (arg === "--phase") {
			const phase = argv[i + 1];
			if (!phase || !(phase in phaseLabels)) {
				throw new Error(`Invalid phase "${phase ?? ""}".`);
			}
			options.phase = phase as PhaseKey;
			i += 1;
		} else if (arg === "--evaluate-batch-ceiling") {
			options.evaluateBatchCeiling = true;
		} else if (arg === "--chosen-limit-pages") {
			const rawValue = argv[i + 1];
			const value = Number(rawValue);
			if (!rawValue || !Number.isFinite(value) || value <= 0) {
				throw new Error(`Invalid page limit "${rawValue ?? ""}".`);
			}
			options.chosenLimitPages = value;
			i += 1;
		} else if (arg === "--batch-pages") {
			const rawValue = argv[i + 1];
			if (!rawValue) {
				throw new Error("Missing required value for --batch-pages.");
			}
			options.batchPages = parseNumberList(rawValue);
			i += 1;
		} else if (arg === "--fresh-engine") {
			options.freshEngine = true;
		} else if (arg === "--render-only") {
			options.renderOnly = true;
		} else if (arg === "--help" || arg === "-h") {
			printUsage();
			process.exit(0);
		} else {
			throw new Error(`Unknown argument: ${arg}`);
		}
	}

	if (options.renderOnly) {
		if (options.phase || options.evaluateBatchCeiling) {
			throw new Error("--render-only cannot be combined with benchmark recording options.");
		}
		return options;
	}

	if (options.phase && options.evaluateBatchCeiling) {
		throw new Error("Choose either --phase or --evaluate-batch-ceiling, not both.");
	}
	if (!options.phase && !options.evaluateBatchCeiling) {
		throw new Error("Missing required --phase or --evaluate-batch-ceiling argument.");
	}
	if (options.evaluateBatchCeiling && !options.chosenLimitPages) {
		throw new Error("--evaluate-batch-ceiling requires --chosen-limit-pages.");
	}

	return options;
}

function formatMs(ms: number): string {
	return `${ms.toFixed(1)}ms`;
}

function formatMultiplier(value: number): string {
	return `${value.toFixed(2)}x`;
}

function formatDelta(value: number, unit = ""): string {
	if (value === 0) {
		return `0${unit}`;
	}

	const sign = value > 0 ? "+" : "";
	return `${sign}${value.toFixed(1)}${unit}`;
}

function formatCountDelta(value: number): string {
	if (value === 0) {
		return "0";
	}

	const sign = value > 0 ? "+" : "";
	return `${sign}${value}`;
}

function formatPercentDelta(current: number, baseline: number): string {
	if (baseline === 0) {
		if (current === 0) {
			return "0.0%";
		}

		return current > 0 ? "+inf%" : "-inf%";
	}

	const delta = ((current - baseline) / baseline) * 100;
	return `${delta > 0 ? "+" : ""}${delta.toFixed(1)}%`;
}

function formatBytes(bytes: number): string {
	const mb = bytes / (1024 * 1024);
	return `${mb.toFixed(2)} MiB`;
}

function formatDataSize(bytes: number): string {
	if (bytes < 1024) {
		return `${bytes} B`;
	}
	if (bytes < 1024 * 1024) {
		return `${(bytes / 1024).toFixed(2)} KiB`;
	}
	return formatBytes(bytes);
}

function formatUs(us: number): string {
	return formatMs(us / 1000);
}

function formatAtomicCoverage(telemetry: SqliteVfsTelemetry): string {
	return [
		`begin ${telemetry.atomicWrite.beginCount}`,
		`commit ${telemetry.atomicWrite.commitAttemptCount}`,
		`ok ${telemetry.atomicWrite.commitSuccessCount}`,
	].join(" / ");
}

function formatDirtyPages(telemetry: SqliteVfsTelemetry): string {
	return [
		`total ${telemetry.atomicWrite.committedDirtyPagesTotal}`,
		`max ${telemetry.atomicWrite.maxCommittedDirtyPages}`,
	].join(" / ");
}

function formatFastPathUsage(telemetry: SqliteVfsTelemetry): string {
	return [
		`attempt ${telemetry.atomicWrite.fastPathAttemptCount ?? 0}`,
		`ok ${telemetry.atomicWrite.fastPathSuccessCount ?? 0}`,
		`fallback ${telemetry.atomicWrite.fastPathFallbackCount ?? 0}`,
		`fail ${telemetry.atomicWrite.fastPathFailureCount ?? 0}`,
	].join(" / ");
}

function formatServerRequestCounts(
	telemetry: SqliteServerTelemetry | undefined,
): string {
	if (!telemetry) {
		return "N/A";
	}

	return [
		`write ${telemetry.writes.requestCount}`,
		`read ${telemetry.reads.requestCount}`,
		`truncate ${telemetry.truncates.requestCount}`,
	].join(" / ");
}

function formatServerDirtyPages(
	telemetry: SqliteServerTelemetry | undefined,
): string {
	if (!telemetry) {
		return "N/A";
	}

	return String(telemetry.writes.dirtyPageCount);
}

function formatServerRequestBytes(
	telemetry: SqliteServerTelemetry | undefined,
): string {
	if (!telemetry) {
		return "N/A";
	}

	return [
		`write ${formatDataSize(telemetry.writes.requestBytes)}`,
		`read ${formatDataSize(telemetry.reads.requestBytes)}`,
		`truncate ${formatDataSize(telemetry.truncates.requestBytes)}`,
	].join(" / ");
}

function formatServerPhaseTiming(
	telemetry: SqliteServerTelemetry | undefined,
): string {
	if (!telemetry) {
		return "N/A";
	}

	return [
		`estimate ${formatUs(telemetry.writes.estimateKvSizeDurationUs)}`,
		`rewrite ${formatUs(telemetry.writes.clearAndRewriteDurationUs)}`,
	].join(" / ");
}

function formatServerValidation(
	telemetry: SqliteServerTelemetry | undefined,
): string {
	if (!telemetry) {
		return "N/A";
	}

	return [
		`ok ${telemetry.writes.validation.ok}`,
		`quota ${telemetry.writes.validation.storageQuotaExceeded}`,
		`payload ${telemetry.writes.validation.payloadTooLarge}`,
		`count ${telemetry.writes.validation.tooManyEntries}`,
	].join(" / ");
}

function renderServerTelemetryDetails(
	telemetry: SqliteServerTelemetry | undefined,
): string {
	if (!telemetry) {
		return "- Server telemetry: unavailable for this run. Re-record it with the current benchmark script.";
	}

	return `- Metrics endpoint: \`${telemetry.metricsEndpoint}\`
- Path label: \`${telemetry.path}\`
- Reads: \`${telemetry.reads.requestCount}\` requests, \`${telemetry.reads.pageEntryCount}\` page keys, \`${telemetry.reads.metadataEntryCount}\` metadata keys, \`${formatDataSize(telemetry.reads.requestBytes)}\` request bytes, \`${formatDataSize(telemetry.reads.responseBytes)}\` response bytes, \`${formatUs(telemetry.reads.durationUs)}\` total
- Writes: \`${telemetry.writes.requestCount}\` requests, \`${telemetry.writes.dirtyPageCount}\` dirty pages, \`${telemetry.writes.metadataEntryCount}\` metadata keys, \`${formatDataSize(telemetry.writes.requestBytes)}\` request bytes, \`${formatDataSize(telemetry.writes.payloadBytes)}\` payload bytes, \`${formatUs(telemetry.writes.durationUs)}\` total
- Path overhead: \`${formatUs(telemetry.writes.estimateKvSizeDurationUs)}\` in \`estimate_kv_size\`, \`${formatUs(telemetry.writes.clearAndRewriteDurationUs)}\` in clear-and-rewrite, \`${telemetry.writes.clearSubspaceCount}\` \`clear_subspace_range\` calls
- Truncates: \`${telemetry.truncates.requestCount}\` requests, \`${formatDataSize(telemetry.truncates.requestBytes)}\` request bytes, \`${formatUs(telemetry.truncates.durationUs)}\` total
- Validation outcomes: \`ok ${telemetry.writes.validation.ok}\` / \`quota ${telemetry.writes.validation.storageQuotaExceeded}\` / \`payload ${telemetry.writes.validation.payloadTooLarge}\` / \`count ${telemetry.writes.validation.tooManyEntries}\` / \`key ${telemetry.writes.validation.keyTooLarge}\` / \`value ${telemetry.writes.validation.valueTooLarge}\` / \`length ${telemetry.writes.validation.lengthMismatch}\``;
}

function buildBenchmarkCommand(
	endpoint: string,
	envOverrides: NodeJS.ProcessEnv = {},
): string {
	const payloadMiB = envOverrides.BENCH_MB ?? process.env.BENCH_MB ?? "10";
	const rowCount = envOverrides.BENCH_ROWS ?? process.env.BENCH_ROWS ?? "1";
	const vars = [
		`BENCH_MB=${payloadMiB}`,
		`BENCH_ROWS=${rowCount}`,
		`RIVET_ENDPOINT=${endpoint}`,
	];
	const readyTimeoutMs =
		envOverrides.BENCH_READY_TIMEOUT_MS ?? process.env.BENCH_READY_TIMEOUT_MS;
	if (readyTimeoutMs) {
		vars.push(`BENCH_READY_TIMEOUT_MS=${readyTimeoutMs}`);
	}
	if (envOverrides.BENCH_REQUIRE_SERVER_TELEMETRY === "1") {
		vars.push("BENCH_REQUIRE_SERVER_TELEMETRY=1");
	}
	return [
		...vars,
		"pnpm --dir examples/sqlite-raw run bench:large-insert -- --json",
	].join(" ");
}

function canonicalWorkflowCommand(options: CliOptions): string {
	if (options.renderOnly) {
		return "pnpm --dir examples/sqlite-raw run bench:record -- --render-only";
	}
	if (options.evaluateBatchCeiling) {
		const args = [
			"--evaluate-batch-ceiling",
			`--chosen-limit-pages ${options.chosenLimitPages}`,
		];
		if (options.batchPages?.length) {
			args.push(`--batch-pages ${options.batchPages.join(",")}`);
		}
		if (options.freshEngine) {
			args.push("--fresh-engine");
		}
		return `pnpm --dir examples/sqlite-raw run bench:record -- ${args.join(" ")}`;
	}

	const args = [`--phase ${options.phase}`];
	if (options.freshEngine) {
		args.push("--fresh-engine");
	}

	return `pnpm --dir examples/sqlite-raw run bench:record -- ${args.join(" ")}`;
}

function canonicalBenchmarkCommand(endpoint: string): string {
	return buildBenchmarkCommand(endpoint);
}

function freshEngineBenchmarkEnv(
	options: CliOptions,
	baseEnv: NodeJS.ProcessEnv = {},
): NodeJS.ProcessEnv {
	if (!options.freshEngine) {
		return baseEnv;
	}

	return {
		...baseEnv,
		BENCH_READY_TIMEOUT_MS:
			baseEnv.BENCH_READY_TIMEOUT_MS ?? defaultFreshEngineReadyTimeoutMs,
	};
}

function runCommand(
	command: string,
	args: string[],
	cwd: string,
	env: NodeJS.ProcessEnv = process.env,
): number {
	const startedAt = performance.now();
	const result = spawnSync(command, args, {
		cwd,
		env,
		stdio: "inherit",
	});
	if (result.status !== 0) {
		throw new Error(
			`${command} ${args.join(" ")} failed with exit code ${result.status ?? "unknown"}.`,
		);
	}
	return performance.now() - startedAt;
}

function selectLatestArtifact(dir: string, suffix: string): string | null {
	if (!existsSync(dir)) {
		return null;
	}

	const matches = readdirSync(dir)
		.filter((entry) => entry.endsWith(suffix))
		.map((entry) => resolve(dir, entry));

	if (matches.length === 0) {
		return null;
	}

	matches.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
	return matches[0] ?? null;
}

function buildProvenance(
	command: string,
	cwd: string,
	durationMs: number,
	artifactPath: string | null,
): BuildProvenance {
	return {
		command,
		cwd: relative(repoRoot, cwd) || ".",
		durationMs,
		artifact: artifactPath ? relative(repoRoot, artifactPath) : null,
		artifactModifiedAt: artifactPath
			? statSync(artifactPath).mtime.toISOString()
			: null,
	};
}

function buildEngine(): BuildProvenance {
	const command = "cargo build --bin rivet-engine";
	const durationMs = runCommand(
		"cargo",
		["build", "--bin", "rivet-engine"],
		repoRoot,
	);
	const binaryName =
		process.platform === "win32" ? "rivet-engine.exe" : "rivet-engine";
	return buildProvenance(
		command,
		repoRoot,
		durationMs,
		resolve(repoRoot, "target/debug", binaryName),
	);
}

function buildNative(): BuildProvenance {
	const nativePackageDir = resolve(
		repoRoot,
		"rivetkit-typescript/packages/rivetkit-native",
	);
	const command =
		"pnpm --dir rivetkit-typescript/packages/rivetkit-native build:force";
	const durationMs = runCommand(
		"pnpm",
		["--dir", nativePackageDir, "run", "build:force"],
		repoRoot,
	);
	return buildProvenance(
		command,
		repoRoot,
		durationMs,
		selectLatestArtifact(nativePackageDir, ".node"),
	);
}

function normalizeHealthUrl(endpoint: string): string {
	return new URL(
		"/health",
		endpoint.endsWith("/") ? endpoint : `${endpoint}/`,
	).toString();
}

function isLocalEndpoint(endpoint: string): boolean {
	const url = new URL(endpoint);
	return (
		url.hostname === "127.0.0.1" ||
		url.hostname === "localhost" ||
		url.hostname === "::1"
	);
}

interface EngineHealth {
	status?: string;
	runtime?: string;
	version?: string;
}

async function fetchHealth(endpoint: string): Promise<EngineHealth | null> {
	try {
		const response = await fetch(normalizeHealthUrl(endpoint), {
			signal: AbortSignal.timeout(1000),
		});
		if (!response.ok) {
			return null;
		}
		return (await response.json()) as EngineHealth;
	} catch {
		return null;
	}
}

async function waitForHealthyEngine(endpoint: string): Promise<void> {
	for (let i = 0; i < 100; i += 1) {
		const health = await fetchHealth(endpoint);
		if (health?.runtime === "engine") {
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}

	throw new Error(`Engine at ${endpoint} did not become healthy within 10s.`);
}

async function assertEngineHealthy(endpoint: string): Promise<void> {
	const health = await fetchHealth(endpoint);
	if (health?.runtime !== "engine") {
		throw new Error(
			`Engine at ${endpoint} is not healthy. Start it first or use --fresh-engine.`,
		);
	}
}

async function startFreshEngine(endpoint: string): Promise<{
	child: ReturnType<typeof spawn>;
	logPath: string;
}> {
	if (!isLocalEndpoint(endpoint)) {
		throw new Error("--fresh-engine only supports local endpoints.");
	}

	const existing = await fetchHealth(endpoint);
	if (existing) {
		throw new Error(
			`Cannot start a fresh engine because ${endpoint} is already serving ${existing.runtime ?? "something"}.`,
		);
	}

	const binaryName =
		process.platform === "win32" ? "rivet-engine.exe" : "rivet-engine";
	const binaryPath = resolve(repoRoot, "target/debug", binaryName);
	const child = spawn(binaryPath, ["start"], {
		cwd: repoRoot,
		stdio: ["ignore", "pipe", "pipe"],
		env: {
			...process.env,
			RUST_BACKTRACE: "full",
			RUST_LOG: process.env.RUST_LOG ?? defaultRustLog,
			RUST_LOG_TARGET: "1",
		},
	});

	if (!child.stdout || !child.stderr) {
		throw new Error(
			"Fresh engine process did not expose stdout/stderr pipes.",
		);
	}

	writeFileSync(defaultLogPath, "");
	child.stdout.on("data", (chunk) => {
		process.stdout.write(chunk);
		writeFileSync(defaultLogPath, chunk, { flag: "a" });
	});
	child.stderr.on("data", (chunk) => {
		process.stderr.write(chunk);
		writeFileSync(defaultLogPath, chunk, { flag: "a" });
	});

	await waitForHealthyEngine(endpoint);
	return { child, logPath: defaultLogPath };
}

function stopFreshEngine(child: ReturnType<typeof spawn>): Promise<void> {
	return new Promise((resolve, reject) => {
		if (child.exitCode !== null) {
			resolve();
			return;
		}

		child.once("exit", () => resolve());
		child.once("error", reject);
		child.kill("SIGTERM");
	});
}

function parseBenchmarkOutput(stdout: string): LargeInsertBenchmarkResult {
	const trimmed = stdout.trim();
	const jsonStart = trimmed.indexOf("{");
	const jsonEnd = trimmed.lastIndexOf("}");

	if (jsonStart === -1 || jsonEnd === -1 || jsonEnd < jsonStart) {
		throw new Error(
			`bench:large-insert did not emit JSON output. Output was:\n${trimmed}`,
		);
	}

	return JSON.parse(
		trimmed.slice(jsonStart, jsonEnd + 1),
	) as LargeInsertBenchmarkResult;
}

function runBenchmark(
	endpoint: string,
	envOverrides: NodeJS.ProcessEnv = {},
): LargeInsertBenchmarkResult {
	const result = spawnSync(
		"pnpm",
		["--dir", exampleDir, "exec", "tsx", "scripts/bench-large-insert.ts", "--", "--json"],
		{
			cwd: repoRoot,
			env: {
				...process.env,
				...envOverrides,
				RIVET_ENDPOINT: endpoint,
			},
			encoding: "utf8",
		},
	);

	if (result.status !== 0) {
		throw new Error(
			result.stderr?.trim() ||
				result.stdout?.trim() ||
				"bench:large-insert failed",
		);
	}

	return parseBenchmarkOutput(result.stdout);
}

function loadStore(): BenchResultsStore {
	if (!existsSync(resultsJsonPath)) {
		return {
			schemaVersion: 1,
			sourceFile: "examples/sqlite-raw/bench-results.json",
			resultsFile: "examples/sqlite-raw/BENCH_RESULTS.md",
			runs: [],
			batchCeilingEvaluations: [],
		};
	}

	return JSON.parse(
		readFileSync(resultsJsonPath, "utf8"),
	) as BenchResultsStore;
}

function saveStore(store: BenchResultsStore): void {
	writeFileSync(resultsJsonPath, `${JSON.stringify(store, null, "\t")}\n`);
}

function latestRunsByPhase(store: BenchResultsStore): Map<PhaseKey, BenchRun> {
	const latest = new Map<PhaseKey, BenchRun>();
	for (const phase of phaseOrder) {
		const run = [...store.runs]
			.reverse()
			.find((candidate) => candidate.phase === phase);
		if (run) {
			latest.set(phase, run);
		}
	}
	return latest;
}

function renderSummaryCell(
	run: BenchRun | undefined,
	value: (candidate: BenchRun) => string,
): string {
	return run ? value(run) : "Pending";
}

function renderBuild(build: BuildProvenance): string {
	const artifact = build.artifact ?? "artifact missing";
	const modifiedAt = build.artifactModifiedAt ?? "mtime unavailable";
	return `- Command: \`${build.command}\`
- CWD: \`${build.cwd}\`
- Artifact: \`${artifact}\`
- Artifact mtime: \`${modifiedAt}\`
- Duration: \`${formatMs(build.durationMs)}\``;
}

function renderPhaseComparison(run: BenchRun, baseline: BenchRun | undefined): string {
	if (!baseline || baseline.id === run.id) {
		return "";
	}

	const currentTelemetry = run.benchmark.actor.vfsTelemetry;
	const baselineTelemetry = baseline.benchmark.actor.vfsTelemetry;
	const actorInsertDelta =
		run.benchmark.actor.insertElapsedMs - baseline.benchmark.actor.insertElapsedMs;
	const actorVerifyDelta =
		run.benchmark.actor.verifyElapsedMs - baseline.benchmark.actor.verifyElapsedMs;
	const endToEndDelta =
		run.benchmark.delta.endToEndElapsedMs -
		baseline.benchmark.delta.endToEndElapsedMs;
	const immediateKvPutDelta =
		currentTelemetry.writes.immediateKvPutCount -
		baselineTelemetry.writes.immediateKvPutCount;
	const batchCapDelta =
		currentTelemetry.atomicWrite.batchCapFailureCount -
		baselineTelemetry.atomicWrite.batchCapFailureCount;

	return `#### Compared to ${phaseLabels[baseline.phase]}

- Atomic write coverage: \`${formatAtomicCoverage(baselineTelemetry)}\` -> \`${formatAtomicCoverage(currentTelemetry)}\`
- Fast-path commit usage: \`${formatFastPathUsage(baselineTelemetry)}\` -> \`${formatFastPathUsage(currentTelemetry)}\`
- Buffered dirty pages: \`${formatDirtyPages(baselineTelemetry)}\` -> \`${formatDirtyPages(currentTelemetry)}\`
- Immediate \`kv_put\` writes: \`${baselineTelemetry.writes.immediateKvPutCount}\` -> \`${currentTelemetry.writes.immediateKvPutCount}\` (\`${formatCountDelta(immediateKvPutDelta)}\`, \`${formatPercentDelta(currentTelemetry.writes.immediateKvPutCount, baselineTelemetry.writes.immediateKvPutCount)}\`)
- Batch-cap failures: \`${baselineTelemetry.atomicWrite.batchCapFailureCount}\` -> \`${currentTelemetry.atomicWrite.batchCapFailureCount}\` (\`${formatCountDelta(batchCapDelta)}\`)
- Actor DB insert: \`${formatMs(baseline.benchmark.actor.insertElapsedMs)}\` -> \`${formatMs(run.benchmark.actor.insertElapsedMs)}\` (\`${formatDelta(actorInsertDelta, "ms")}\`, \`${formatPercentDelta(run.benchmark.actor.insertElapsedMs, baseline.benchmark.actor.insertElapsedMs)}\`)
- Actor DB verify: \`${formatMs(baseline.benchmark.actor.verifyElapsedMs)}\` -> \`${formatMs(run.benchmark.actor.verifyElapsedMs)}\` (\`${formatDelta(actorVerifyDelta, "ms")}\`, \`${formatPercentDelta(run.benchmark.actor.verifyElapsedMs, baseline.benchmark.actor.verifyElapsedMs)}\`)
- End-to-end action: \`${formatMs(baseline.benchmark.delta.endToEndElapsedMs)}\` -> \`${formatMs(run.benchmark.delta.endToEndElapsedMs)}\` (\`${formatDelta(endToEndDelta, "ms")}\`, \`${formatPercentDelta(run.benchmark.delta.endToEndElapsedMs, baseline.benchmark.delta.endToEndElapsedMs)}\`)`;
}

function comparisonBaselinesForRun(
	run: BenchRun,
	latest: Map<PhaseKey, BenchRun>,
): BenchRun[] {
	if (run.phase === "phase-0") {
		return [];
	}

	const baselinePhases: PhaseKey[] =
		run.phase === "phase-1"
			? ["phase-0"]
			: run.phase === "phase-2-3"
				? ["phase-0", "phase-1"]
				: ["phase-0", "phase-1", "phase-2-3"];

	return baselinePhases
		.map((phase) => latest.get(phase))
		.filter((candidate): candidate is BenchRun => {
			return candidate !== undefined && candidate.id !== run.id;
		});
}

function renderHistoricalReference(): string {
	return `## Historical Reference

The section below predates this scaffold. Keep it for context, but append new
phase results through \`bench-results.json\` and \`bench:record\`.

### 2026-04-15 Exploratory Large Insert Runs

| Payload | Actor DB Insert | Actor DB Verify | End-to-End Action | Native SQLite Insert | Actor DB vs Native | End-to-End vs Native |
| ------- | --------------- | --------------- | ----------------- | -------------------- | ------------------ | -------------------- |
| 1 MiB   | 832.2ms         | 0.4ms           | 1137.6ms          | 1.8ms                | 461.11x            | 630.34x              |
| 5 MiB   | 4199.6ms        | 3655.5ms        | 8186.3ms          | 25.3ms               | 166.19x            | 323.96x              |
| 10 MiB  | 9438.2ms        | 8973.5ms        | 19244.0ms         | 45.5ms               | 207.34x            | 422.75x              |

- Command: \`pnpm --dir examples/sqlite-raw bench:large-insert\`
- Additional runs: \`BENCH_MB=1\`, \`BENCH_MB=5\`, \`BENCH_MB=10\`, and one
  \`RUST_LOG=rivetkit_sqlite_native::vfs=debug BENCH_MB=1\` trace run.
- Debug trace clue: 317 total KV round-trips, 30 \`get(...)\` calls,
  287 \`put(...)\` calls, 577 total keys written, 63.1ms traced \`get\` time,
  and 856.0ms traced \`put\` time.
- Conclusion: the bottleneck already looked like SQLite-over-KV page churn,
  not raw SQLite execution.
`;
}

function renderBatchCeilingEvaluation(
	evaluation: BatchCeilingEvaluation,
): string {
	const rows = evaluation.samples
		.map((sample) => {
			const path =
				(sample.benchmark.actor.vfsTelemetry.atomicWrite
					.fastPathSuccessCount ?? 0) > 0
					? "fast_path"
					: sample.benchmark.serverTelemetry?.path ?? "N/A";
			const dirtyPages =
				sample.benchmark.actor.vfsTelemetry.atomicWrite
					.maxFastPathDirtyPages ??
				sample.benchmark.serverTelemetry?.writes.dirtyPageCount ??
				sample.benchmark.actor.vfsTelemetry.atomicWrite.maxCommittedDirtyPages;
			const requestBytes =
				sample.benchmark.actor.vfsTelemetry.atomicWrite
					.maxFastPathRequestBytes ??
				sample.benchmark.serverTelemetry?.writes.requestBytes ?? 0;
			const commitLatencyUs =
				sample.benchmark.actor.vfsTelemetry.atomicWrite
					.maxFastPathDurationUs ??
				sample.benchmark.serverTelemetry?.writes.durationUs ??
				sample.benchmark.actor.vfsTelemetry.atomicWrite.commitDurationUs;

			return `| ${sample.targetDirtyPages} | ${sample.payloadMiB.toFixed(2)} MiB | ${path} | ${dirtyPages} | ${formatDataSize(requestBytes)} | ${formatUs(commitLatencyUs)} | ${formatMs(sample.benchmark.actor.insertElapsedMs)} |`;
		})
		.join("\n");
	const notes = evaluation.notes.map((note) => `- ${note}`).join("\n");

	return `### ${evaluation.recordedAt}

- Chosen SQLite fast-path ceiling: \`${evaluation.chosenLimitPages}\` dirty pages
- Generic actor-KV cap: \`128\` entries
- Workflow command: \`${evaluation.workflowCommand}\`
- Endpoint: \`${evaluation.endpoint}\`
- Fresh engine start: \`${evaluation.freshEngineStart ? "yes" : "no"}\`
- Engine log: \`${evaluation.engineLogPath ?? "not captured"}\`
- Notes:
${notes}

| Target pages | Payload | Path | Actual dirty pages | Request bytes | Commit latency | Actor DB insert |
| --- | --- | --- | --- | --- | --- | --- |
${rows}

#### Engine Build Provenance

${renderBuild(evaluation.engineBuild)}

#### Native Build Provenance

${renderBuild(evaluation.nativeBuild)}`;
}

function renderBatchCeilingEvaluations(store: BenchResultsStore): string {
	const evaluations = [...(store.batchCeilingEvaluations ?? [])].reverse();
	if (evaluations.length === 0) {
		return "No batch ceiling evaluations recorded yet.";
	}

	const [latestEvaluation] = evaluations;
	const historicalNote =
		evaluations.length > 1
			? "\n\nOlder evaluations remain in `bench-results.json`; the latest successful rerun is rendered here."
			: "";

	return `${renderBatchCeilingEvaluation(latestEvaluation)}${historicalNote}`;
}

function renderMarkdown(store: BenchResultsStore): string {
	const latest = latestRunsByPhase(store);
	const summaryRows = [
		[
			"Status",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), () => "Recorded"),
			),
		],
		[
			"Recorded at",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) => run.recordedAt),
			),
		],
		[
			"Git SHA",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					run.gitSha.slice(0, 12),
				),
			),
		],
		[
			"Fresh engine",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					run.freshEngineStart ? "yes" : "no",
				),
			),
		],
		[
			"Payload",
			...phaseOrder.map((phase) =>
				renderSummaryCell(
					latest.get(phase),
					(run) => `${run.benchmark.payloadMiB} MiB`,
				),
			),
		],
		[
			"Rows",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					String(run.benchmark.rowCount),
				),
			),
		],
		[
			"Atomic write coverage",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatAtomicCoverage(run.benchmark.actor.vfsTelemetry),
				),
			),
		],
		[
			"Buffered dirty pages",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatDirtyPages(run.benchmark.actor.vfsTelemetry),
				),
			),
		],
		[
			"Immediate kv_put writes",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					String(
						run.benchmark.actor.vfsTelemetry.writes
							.immediateKvPutCount,
					),
				),
			),
		],
		[
			"Batch-cap failures",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					String(
						run.benchmark.actor.vfsTelemetry.atomicWrite
							.batchCapFailureCount,
					),
				),
			),
		],
		[
			"Server request counts",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatServerRequestCounts(run.benchmark.serverTelemetry),
				),
			),
		],
		[
			"Server dirty pages",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatServerDirtyPages(run.benchmark.serverTelemetry),
				),
			),
		],
		[
			"Server request bytes",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatServerRequestBytes(run.benchmark.serverTelemetry),
				),
			),
		],
		[
			"Server overhead timing",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatServerPhaseTiming(run.benchmark.serverTelemetry),
				),
			),
		],
		[
			"Server validation",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatServerValidation(run.benchmark.serverTelemetry),
				),
			),
		],
		[
			"Actor DB insert",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatMs(run.benchmark.actor.insertElapsedMs),
				),
			),
		],
		[
			"Actor DB verify",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatMs(run.benchmark.actor.verifyElapsedMs),
				),
			),
		],
		[
			"End-to-end action",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatMs(run.benchmark.delta.endToEndElapsedMs),
				),
			),
		],
		[
			"Native SQLite insert",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatMs(run.benchmark.native.insertElapsedMs),
				),
			),
		],
		[
			"Actor DB vs native",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatMultiplier(
						run.benchmark.delta.actorDbVsNativeMultiplier,
					),
				),
			),
		],
		[
			"End-to-end vs native",
			...phaseOrder.map((phase) =>
				renderSummaryCell(latest.get(phase), (run) =>
					formatMultiplier(
						run.benchmark.delta.endToEndVsNativeMultiplier,
					),
				),
			),
		],
	]
		.map(([metric, ...values]) => `| ${metric} | ${values.join(" | ")} |`)
		.join("\n");

	const runLog = [...store.runs]
		.reverse()
		.map((run) => {
			const phaseComparisons = comparisonBaselinesForRun(run, latest)
				.map((baseline) => renderPhaseComparison(run, baseline))
				.filter((comparison) => comparison.length > 0)
				.join("\n\n");
			const phaseComparisonSection = phaseComparisons
				? `\n\n${phaseComparisons}`
				: "";

			return `### ${phaseLabels[run.phase]} · ${run.recordedAt}

- Run ID: \`${run.id}\`
- Git SHA: \`${run.gitSha}\`
- Workflow command: \`${run.workflowCommand}\`
- Benchmark command: \`${run.benchmarkCommand}\`
- Endpoint: \`${run.endpoint}\`
- Fresh engine start: \`${run.freshEngineStart ? "yes" : "no"}\`
- Engine log: \`${run.engineLogPath ?? "not captured"}\`
- Payload: \`${run.benchmark.payloadMiB} MiB\`
- Total bytes: \`${formatBytes(run.benchmark.totalBytes)}\`
- Rows: \`${run.benchmark.rowCount}\`
- Actor DB insert: \`${formatMs(run.benchmark.actor.insertElapsedMs)}\`
- Actor DB verify: \`${formatMs(run.benchmark.actor.verifyElapsedMs)}\`
- End-to-end action: \`${formatMs(run.benchmark.delta.endToEndElapsedMs)}\`
- Native SQLite insert: \`${formatMs(run.benchmark.native.insertElapsedMs)}\`
- Actor DB vs native: \`${formatMultiplier(run.benchmark.delta.actorDbVsNativeMultiplier)}\`
- End-to-end vs native: \`${formatMultiplier(run.benchmark.delta.endToEndVsNativeMultiplier)}\`${phaseComparisonSection}

#### VFS Telemetry

- Reads: \`${run.benchmark.actor.vfsTelemetry.reads.count}\` calls, \`${formatBytes(run.benchmark.actor.vfsTelemetry.reads.returnedBytes)}\` returned, \`${run.benchmark.actor.vfsTelemetry.reads.shortReadCount}\` short reads, \`${formatUs(run.benchmark.actor.vfsTelemetry.reads.durationUs)}\` total
- Writes: \`${run.benchmark.actor.vfsTelemetry.writes.count}\` calls, \`${formatBytes(run.benchmark.actor.vfsTelemetry.writes.inputBytes)}\` input, \`${run.benchmark.actor.vfsTelemetry.writes.bufferedCount}\` buffered calls, \`${run.benchmark.actor.vfsTelemetry.writes.immediateKvPutCount}\` immediate \`kv_put\` fallbacks
- Syncs: \`${run.benchmark.actor.vfsTelemetry.syncs.count}\` calls, \`${run.benchmark.actor.vfsTelemetry.syncs.metadataFlushCount}\` metadata flushes, \`${formatUs(run.benchmark.actor.vfsTelemetry.syncs.durationUs)}\` total
- Atomic write coverage: \`${formatAtomicCoverage(run.benchmark.actor.vfsTelemetry)}\`
- Fast-path commit usage: \`${formatFastPathUsage(run.benchmark.actor.vfsTelemetry)}\`
- Atomic write pages: \`${formatDirtyPages(run.benchmark.actor.vfsTelemetry)}\`
- Atomic write bytes: \`${formatBytes(run.benchmark.actor.vfsTelemetry.atomicWrite.committedBufferedBytesTotal)}\`
- Atomic write failures: \`${run.benchmark.actor.vfsTelemetry.atomicWrite.batchCapFailureCount}\` batch-cap, \`${run.benchmark.actor.vfsTelemetry.atomicWrite.commitKvPutFailureCount}\` KV put
- KV round-trips: \`get ${run.benchmark.actor.vfsTelemetry.kv.getCount}\` / \`put ${run.benchmark.actor.vfsTelemetry.kv.putCount}\` / \`delete ${run.benchmark.actor.vfsTelemetry.kv.deleteCount}\` / \`deleteRange ${run.benchmark.actor.vfsTelemetry.kv.deleteRangeCount}\`
- KV payload bytes: \`${formatBytes(run.benchmark.actor.vfsTelemetry.kv.getBytes)}\` read, \`${formatBytes(run.benchmark.actor.vfsTelemetry.kv.putBytes)}\` written

#### Server Telemetry

${renderServerTelemetryDetails(run.benchmark.serverTelemetry)}

#### Engine Build Provenance

${renderBuild(run.engineBuild)}

#### Native Build Provenance

${renderBuild(run.nativeBuild)}`;
		})
		.join("\n\n");

	return `# SQLite Large Insert Results

This file is generated from \`bench-results.json\` by
\`pnpm --dir examples/sqlite-raw run bench:record -- --render-only\`.

## Source of Truth

- Structured runs live in \`examples/sqlite-raw/bench-results.json\`.
- The rendered summary lives in \`examples/sqlite-raw/BENCH_RESULTS.md\`.
- Later phases should append by rerunning \`bench:record\`, not by inventing a
  new markdown format.

## Phase Summary

| Metric | ${phaseOrder.map((phase) => phaseLabels[phase]).join(" | ")} |
| --- | --- | --- | --- | --- |
${summaryRows}

## SQLite Fast-Path Batch Ceiling

${renderBatchCeilingEvaluations(store)}

## Append-Only Run Log

${runLog || "No structured runs recorded yet."}

${renderHistoricalReference()}`;
}

function writeMarkdown(store: BenchResultsStore): void {
	writeFileSync(resultsMarkdownPath, `${renderMarkdown(store)}\n`);
}

function recordRun(store: BenchResultsStore, run: BenchRun): BenchResultsStore {
	return {
		...store,
		runs: [...store.runs, run],
	};
}

function recordBatchCeilingEvaluation(
	store: BenchResultsStore,
	evaluation: BatchCeilingEvaluation,
): BenchResultsStore {
	return {
		...store,
		batchCeilingEvaluations: [
			...(store.batchCeilingEvaluations ?? []),
			evaluation,
		],
	};
}

function payloadMiBForTargetDirtyPages(targetDirtyPages: number): number {
	const payloadBytes =
		Math.max(1, targetDirtyPages - sqlitePageOverheadEstimate) *
		sqlitePageSizeBytes;
	return Number((payloadBytes / (1024 * 1024)).toFixed(2));
}

async function main(): Promise<void> {
	const options = parseArgs(process.argv.slice(2));
	const store = loadStore();

	if (options.renderOnly) {
		saveStore(store);
		writeMarkdown(store);
		console.log(`Rendered ${relative(repoRoot, resultsMarkdownPath)}.`);
		return;
	}

	const endpoint = defaultEndpoint;
	let engineChild: ReturnType<typeof spawn> | null = null;
	let engineLogPath: string | null = null;

	try {
		const phase = options.phase;

		const gitSha = execFileSync("git", ["rev-parse", "HEAD"], {
			cwd: repoRoot,
			encoding: "utf8",
		}).trim();
		const engineBuild = buildEngine();
		const nativeBuild = buildNative();

		if (options.freshEngine) {
			const fresh = await startFreshEngine(endpoint);
			engineChild = fresh.child;
			engineLogPath = fresh.logPath;
		} else {
			await assertEngineHealthy(endpoint);
		}

		let nextStore = store;
		if (options.evaluateBatchCeiling) {
			const chosenLimitPages = options.chosenLimitPages!;
			const batchPages = options.batchPages?.length
				? options.batchPages
				: [...defaultBatchCeilingPages];
			if (!batchPages.includes(chosenLimitPages)) {
				batchPages.push(chosenLimitPages);
				batchPages.sort((a, b) => a - b);
			}

			const samples: BatchCeilingSample[] = [];
			for (const targetDirtyPages of batchPages) {
				const payloadMiB = payloadMiBForTargetDirtyPages(targetDirtyPages);
				const benchmarkEnv = freshEngineBenchmarkEnv(options, {
					BENCH_MB: payloadMiB.toFixed(2),
					BENCH_REQUIRE_SERVER_TELEMETRY: "1",
				});
				samples.push({
					targetDirtyPages,
					payloadMiB,
					benchmarkCommand: buildBenchmarkCommand(endpoint, benchmarkEnv),
					benchmark: runBenchmark(endpoint, benchmarkEnv),
				});
			}

			const evaluation: BatchCeilingEvaluation = {
				id: `batch-ceiling-${Date.now()}`,
				recordedAt: new Date().toISOString(),
				gitSha,
				workflowCommand: canonicalWorkflowCommand(options),
				endpoint,
				freshEngineStart: options.freshEngine,
				engineLogPath,
				engineBuild,
				nativeBuild,
				chosenLimitPages,
				batchPages,
				notes: [
					"These samples measure the SQLite fast path above the generic 128-entry actor-KV cap on the local benchmark engine.",
					"The local benchmark path reports request bytes and commit latency from VFS fast-path telemetry because pegboard metrics stay zero when the actor runs in-process.",
					"Engine config still defaults envoy tunnel payloads to 20 MiB, so request bytes should stay comfortably below that envelope before raising the ceiling again.",
				],
				samples,
			};

			nextStore = recordBatchCeilingEvaluation(store, evaluation);
		} else {
			if (!phase) {
				throw new Error("Missing required phase.");
			}
			const benchmarkEnv = freshEngineBenchmarkEnv(options);
			const benchmark = runBenchmark(endpoint, benchmarkEnv);
			const run: BenchRun = {
				id: `${phase}-${Date.now()}`,
				phase,
				recordedAt: new Date().toISOString(),
				gitSha,
				workflowCommand: canonicalWorkflowCommand(options),
				benchmarkCommand: buildBenchmarkCommand(endpoint, benchmarkEnv),
				endpoint,
				freshEngineStart: options.freshEngine,
				engineLogPath,
				engineBuild,
				nativeBuild,
				benchmark,
			};

			nextStore = recordRun(store, run);
		}
		saveStore(nextStore);
		writeMarkdown(nextStore);

		if (options.evaluateBatchCeiling) {
			console.log(
				`Recorded SQLite fast-path batch ceiling evaluation in ${relative(repoRoot, resultsJsonPath)}.`,
			);
		} else {
			if (!phase) {
				throw new Error("Missing required phase.");
			}
			console.log(
				`Recorded ${phaseLabels[phase]} benchmark in ${relative(repoRoot, resultsJsonPath)}.`,
			);
		}
	} finally {
		if (engineChild) {
			await stopFreshEngine(engineChild);
		}
	}
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
