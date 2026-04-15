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
const defaultEndpoint = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const defaultLogPath = "/tmp/sqlite-raw-bench-engine.log";
const defaultRustLog =
	"opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug";

type PhaseKey = (typeof phaseOrder)[number];

interface CliOptions {
	phase?: PhaseKey;
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
	path: "generic";
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

interface BenchResultsStore {
	schemaVersion: 1;
	sourceFile: string;
	resultsFile: string;
	runs: BenchRun[];
}

function printUsage(): void {
	console.log(`Usage:
  pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-0 [--fresh-engine]
  pnpm --dir examples/sqlite-raw run bench:record -- --render-only

Options:
  --phase <phase-0|phase-1|phase-2-3|final>
  --fresh-engine   Build and start a fresh local engine before the benchmark
  --render-only    Regenerate BENCH_RESULTS.md from bench-results.json

Environment:
  BENCH_MB         Payload size in MiB. Defaults to 10.
  BENCH_ROWS       Number of rows. Defaults to 1.
  RIVET_ENDPOINT   Engine endpoint. Defaults to http://127.0.0.1:6420.
`);
}

function parseArgs(argv: string[]): CliOptions {
	const options: CliOptions = {
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

	if (!options.renderOnly && !options.phase) {
		throw new Error("Missing required --phase argument.");
	}

	return options;
}

function formatMs(ms: number): string {
	return `${ms.toFixed(1)}ms`;
}

function formatMultiplier(value: number): string {
	return `${value.toFixed(2)}x`;
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
- Generic overhead: \`${formatUs(telemetry.writes.estimateKvSizeDurationUs)}\` in \`estimate_kv_size\`, \`${formatUs(telemetry.writes.clearAndRewriteDurationUs)}\` in clear-and-rewrite, \`${telemetry.writes.clearSubspaceCount}\` \`clear_subspace_range\` calls
- Truncates: \`${telemetry.truncates.requestCount}\` requests, \`${formatDataSize(telemetry.truncates.requestBytes)}\` request bytes, \`${formatUs(telemetry.truncates.durationUs)}\` total
- Validation outcomes: \`ok ${telemetry.writes.validation.ok}\` / \`quota ${telemetry.writes.validation.storageQuotaExceeded}\` / \`payload ${telemetry.writes.validation.payloadTooLarge}\` / \`count ${telemetry.writes.validation.tooManyEntries}\` / \`key ${telemetry.writes.validation.keyTooLarge}\` / \`value ${telemetry.writes.validation.valueTooLarge}\` / \`length ${telemetry.writes.validation.lengthMismatch}\``;
}

function canonicalWorkflowCommand(options: CliOptions): string {
	if (options.renderOnly) {
		return "pnpm --dir examples/sqlite-raw run bench:record -- --render-only";
	}

	const args = [`--phase ${options.phase}`];
	if (options.freshEngine) {
		args.push("--fresh-engine");
	}

	return `pnpm --dir examples/sqlite-raw run bench:record -- ${args.join(" ")}`;
}

function canonicalBenchmarkCommand(endpoint: string): string {
	const payloadMiB = process.env.BENCH_MB ?? "10";
	const rowCount = process.env.BENCH_ROWS ?? "1";
	return [
		`BENCH_MB=${payloadMiB}`,
		`BENCH_ROWS=${rowCount}`,
		`RIVET_ENDPOINT=${endpoint}`,
		"pnpm --dir examples/sqlite-raw run bench:large-insert -- --json",
	].join(" ");
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

function runBenchmark(endpoint: string): LargeInsertBenchmarkResult {
	const result = spawnSync(
		"pnpm",
		["--dir", exampleDir, "exec", "tsx", "scripts/bench-large-insert.ts", "--", "--json"],
		{
			cwd: repoRoot,
			env: {
				...process.env,
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
- End-to-end vs native: \`${formatMultiplier(run.benchmark.delta.endToEndVsNativeMultiplier)}\`

#### VFS Telemetry

- Reads: \`${run.benchmark.actor.vfsTelemetry.reads.count}\` calls, \`${formatBytes(run.benchmark.actor.vfsTelemetry.reads.returnedBytes)}\` returned, \`${run.benchmark.actor.vfsTelemetry.reads.shortReadCount}\` short reads, \`${formatUs(run.benchmark.actor.vfsTelemetry.reads.durationUs)}\` total
- Writes: \`${run.benchmark.actor.vfsTelemetry.writes.count}\` calls, \`${formatBytes(run.benchmark.actor.vfsTelemetry.writes.inputBytes)}\` input, \`${run.benchmark.actor.vfsTelemetry.writes.bufferedCount}\` buffered calls, \`${run.benchmark.actor.vfsTelemetry.writes.immediateKvPutCount}\` immediate \`kv_put\` fallbacks
- Syncs: \`${run.benchmark.actor.vfsTelemetry.syncs.count}\` calls, \`${run.benchmark.actor.vfsTelemetry.syncs.metadataFlushCount}\` metadata flushes, \`${formatUs(run.benchmark.actor.vfsTelemetry.syncs.durationUs)}\` total
- Atomic write coverage: \`${formatAtomicCoverage(run.benchmark.actor.vfsTelemetry)}\`
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
		if (!phase) {
			throw new Error("Missing required phase.");
		}

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

		const benchmark = runBenchmark(endpoint);
		const run: BenchRun = {
			id: `${phase}-${Date.now()}`,
			phase,
			recordedAt: new Date().toISOString(),
			gitSha,
			workflowCommand: canonicalWorkflowCommand(options),
			benchmarkCommand: canonicalBenchmarkCommand(endpoint),
			endpoint,
			freshEngineStart: options.freshEngine,
			engineLogPath,
			engineBuild,
			nativeBuild,
			benchmark,
		};

		const nextStore = recordRun(store, run);
		saveStore(nextStore);
		writeMarkdown(nextStore);

		console.log(
			`Recorded ${phaseLabels[run.phase]} benchmark in ${relative(repoRoot, resultsJsonPath)}.`,
		);
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
