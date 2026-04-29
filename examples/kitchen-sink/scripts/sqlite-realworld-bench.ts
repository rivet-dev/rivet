#!/usr/bin/env -S pnpm exec tsx

import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const DEFAULT_ENDPOINT = "http://127.0.0.1:6420";
const DEFAULT_WAKE_DELAY_MS = 2000;
const DEFAULT_POST_SETUP_WAIT_MS = 0;
const DEFAULT_ROW_BYTES = 2 * 1024;
const SQLITE_PAGE_SIZE_BYTES = 4096;
const DEFAULT_STARTUP_PRELOAD_MAX_BYTES = 1024 * 1024;
const DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES = 50_000;
const REPO_ENGINE_BINARY = fileURLToPath(
	new URL("../../../target/debug/rivet-engine", import.meta.url),
);
const REPO_ROOT = fileURLToPath(new URL("../../..", import.meta.url));
const DEFAULT_RESULTS_ROOT = ".agent/benchmarks/sqlite-realworld";
const SQLITE_OPT_MODE_ENVS = [
	"RIVETKIT_SQLITE_OPT_READ_AHEAD_MODE",
	"RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_MODE",
] as const;
const SQLITE_OPT_BOOLEAN_ENVS = [
	"RIVETKIT_SQLITE_OPT_RECENT_PAGE_HINTS",
	"RIVETKIT_SQLITE_OPT_PRELOAD_HINT_FLUSH",
	"RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_FIRST_PAGES",
	"RIVETKIT_SQLITE_OPT_PRELOAD_HINTS_ON_OPEN",
	"RIVETKIT_SQLITE_OPT_PRELOAD_HINT_HOT_PAGES",
	"RIVETKIT_SQLITE_OPT_PRELOAD_HINT_EARLY_PAGES",
	"RIVETKIT_SQLITE_OPT_PRELOAD_HINT_SCAN_RANGES",
	"RIVETKIT_SQLITE_OPT_CACHE_GET_PAGES_VALIDATION",
	"RIVETKIT_SQLITE_OPT_RANGE_READS",
	"RIVETKIT_SQLITE_OPT_BATCH_CHUNK_READS",
	"RIVETKIT_SQLITE_OPT_DECODED_LTX_CACHE",
	"RIVETKIT_SQLITE_OPT_READ_POOL_ENABLED",
] as const;
const SQLITE_OPT_NUMERIC_ENVS = [
	"RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_MAX_BYTES",
	"RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_FIRST_PAGE_COUNT",
	"RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_CAPACITY_PAGES",
	"RIVETKIT_SQLITE_OPT_VFS_PROTECTED_CACHE_PAGES",
	"RIVETKIT_SQLITE_OPT_READ_POOL_MAX_READERS",
	"RIVETKIT_SQLITE_OPT_READ_POOL_IDLE_TTL_MS",
] as const;

const WORKLOADS = [
	"small-rowid-point",
	"small-schema-read",
	"small-range-scan",
	"rowid-range-forward",
	"rowid-range-backward",
	"secondary-index-covering-range",
	"secondary-index-scattered-table",
	"aggregate-status",
	"aggregate-time-bucket",
	"aggregate-tenant-time-range",
	"parallel-read-aggregates",
	"parallel-read-write-transition",
	"feed-order-by-limit",
	"feed-pagination-adjacent",
	"join-order-items",
	"random-point-lookups",
	"hot-index-cold-table",
	"ledger-without-rowid-range",
	"write-batch-after-wake",
	"update-hot-partition",
	"delete-churn-range-read",
	"migration-create-indexes-large",
	"migration-create-indexes-skewed-large",
	"migration-table-rebuild-large",
	"migration-add-column-large",
	"migration-ddl-small",
] as const;

type WorkloadName = (typeof WORKLOADS)[number];
type SizeClass = "none" | "small" | "medium" | "cache-fit" | "cache-overflow" | "large";
type Profile = "standard" | "smoke";
type WorkloadCategory = "read" | "write" | "migration" | "canary";

interface Args {
	endpoint: string;
	key: string;
	profile: Profile;
	only: WorkloadName[];
	smallBytes: number;
	mediumBytes: number;
	cacheFitBytes: number;
	cacheOverflowBytes: number;
	largeBytes: number;
	rowBytes: number;
	wakeDelayMs: number;
	postSetupWaitMs: number;
	outputDir: string;
	metricsToken: string;
	disableMetadataLookup: boolean;
	startLocalEnvoy: boolean;
	disableStorageCompaction: boolean;
	disableSqliteOptimizations: boolean;
}

interface LocalEngine {
	child: ChildProcess;
	dbRoot: string;
	logs: string[];
}

interface WorkloadSpec {
	name: WorkloadName;
	category: WorkloadCategory;
	sizeClass: SizeClass;
	description: string;
}

interface SetupResult {
	rows: number;
	targetBytes: number;
	rowBytes: number;
	setupMs: number;
	pageCount: number;
}

interface MainResult {
	ms: number;
	workload: WorkloadName;
	pageCount: number;
	[key: string]: unknown;
}

interface CacheConfigResult {
	sqliteCacheSizePragma: number | null;
	sqlitePageSize: number | null;
	pageCount: number;
}

interface BenchmarkResult {
	workload: WorkloadName;
	description: string;
	category: WorkloadCategory;
	sizeClass: SizeClass;
	targetBytes: number;
	actorKey: string[];
	actorId: string;
	setup: SetupResult | null;
	main: MainResult;
	vfsMetrics: VfsMetricSnapshot;
}

interface VfsMetricSnapshot {
	resolvePagesTotal: number;
	resolvePagesRequestedTotal: number;
	resolvePagesCacheHitsTotal: number;
	resolvePagesCacheMissesTotal: number;
	getPagesTotal: number;
	pagesFetchedTotal: number;
	prefetchPagesTotal: number;
	bytesFetchedTotal: number;
	prefetchBytesTotal: number;
	getPagesDurationSecondsSum: number;
	getPagesDurationSecondsCount: number;
}

const WORKLOAD_SPECS: WorkloadSpec[] = [
	{
		// Included to keep tiny actor databases honest while we optimize larger datasets.
		// Startup preload and the first few VFS pages should cover most metadata, and the point reads should be page-cache friendly.
		name: "small-rowid-point",
		category: "canary",
		sizeClass: "small",
		description: "Small cold-wake primary-key point reads.",
	},
	{
		// Included because many apps hit schema/catalog pages immediately after opening SQLite.
		// Schema and root pages should be strong startup preload and protected-cache candidates.
		name: "small-schema-read",
		category: "canary",
		sizeClass: "small",
		description: "Small cold-wake schema and table metadata read.",
	},
	{
		// Included to verify read-ahead and preload logic do not add overhead to small table scans.
		// The full dataset should fit in cache, so read-ahead should stay cheap and avoid material overfetch.
		name: "small-range-scan",
		category: "canary",
		sizeClass: "small",
		description: "Small rowid range scan to catch regressions on tiny databases.",
	},
	{
		// Included for append-heavy product tables where rowid order often maps well to physical page locality.
		// Adaptive forward read-ahead should reduce VFS round trips with little random access.
		name: "rowid-range-forward",
		category: "read",
		sizeClass: "large",
		description: "Large append-like INTEGER PRIMARY KEY forward range scan.",
	},
	{
		// Included because feeds and history views often scan newest-to-oldest.
		// Backward read-ahead should detect decreasing page access and avoid many small fetches.
		name: "rowid-range-backward",
		category: "read",
		sizeClass: "large",
		description: "Large append-like INTEGER PRIMARY KEY reverse range scan.",
	},
	{
		// Included to isolate index-only range reads from table hydration.
		// Most work should stay in index pages, giving compact access compared with non-covered table reads.
		name: "secondary-index-covering-range",
		category: "read",
		sizeClass: "large",
		description: "Large covering secondary-index range scan.",
	},
	{
		// Included to model secondary-index lookup plus record hydration when index order is not table-page order.
		// Cache hit rate should be worse than the covering case, and read-ahead should avoid overcommitting on scattered table-page reads.
		name: "secondary-index-scattered-table",
		category: "read",
		sizeClass: "large",
		description: "Large secondary-index range that visits table rows in scattered rowid order.",
	},
	{
		// Included for actor-local reporting over operational tables.
		// This should be scan-heavy: read-ahead should help, while cache capacity only helps if the table fits or pages are revisited.
		name: "aggregate-status",
		category: "read",
		sizeClass: "large",
		description: "Large GROUP BY status aggregate over an OLTP-style orders table.",
	},
	{
		// Included for dashboard-style time bucketing across many rows.
		// Mostly sequential table/index access should benefit from read-ahead, but computed grouping should not depend on warm pager state.
		name: "aggregate-time-bucket",
		category: "read",
		sizeClass: "large",
		description: "Large time-bucket aggregate over an OLTP-style orders table.",
	},
	{
		// Included for selective OLTP aggregates scoped to one tenant and time range.
		// The event index should narrow the scan, while joins back to orders expose table-page reuse and scattered lookup cost.
		name: "aggregate-tenant-time-range",
		category: "read",
		sizeClass: "cache-fit",
		description: "Selective tenant/time-range aggregate over events joined to orders.",
	},
	{
		// Included to measure future read-mode parallelism where several read-only SQLite connections overlap VFS misses.
		// Today this captures the serialized baseline; after the connection manager lands, independent aggregate reads should overlap.
		name: "parallel-read-aggregates",
		category: "read",
		sizeClass: "large",
		description: "Concurrent read-only aggregates over one actor-local SQLite database.",
	},
	{
		// Included to measure the read-mode to write-mode transition.
		// Future write mode must wait for active readers, close them, run exactly one writable connection, then allow fresh readers.
		name: "parallel-read-write-transition",
		category: "write",
		sizeClass: "medium",
		description: "Concurrent read aggregates with a queued write-mode update.",
	},
	{
		// Included for the first page of a timeline, inbox, or event feed after actor wake.
		// Recent index/root pages should be good preload candidates, and LIMIT should keep fetched table pages bounded.
		name: "feed-order-by-limit",
		category: "read",
		sizeClass: "medium",
		description: "Recent-feed ORDER BY indexed timestamp with LIMIT.",
	},
	{
		// Included to test adjacent cursor pages, not just the first feed page.
		// The second page should reuse nearby index pages and stay bounded by LIMIT rather than scanning the whole table.
		name: "feed-pagination-adjacent",
		category: "read",
		sizeClass: "medium",
		description: "Adjacent cursor pagination over an indexed recent-feed query.",
	},
	{
		// Included because joins can bounce between parent and child B-trees.
		// Page cache should preserve hot parent/index pages while child table scans may still benefit from read-ahead.
		name: "join-order-items",
		category: "read",
		sizeClass: "large",
		description: "Orders to order-items join with grouped totals.",
	},
	{
		// Included as a non-scan workload to catch optimizations that only help sequential reads.
		// Read-ahead should stay bounded; cache wins should come from repeated root/index page reuse.
		name: "random-point-lookups",
		category: "read",
		sizeClass: "large",
		description: "Deterministic random primary-key point lookups across a large table.",
	},
	{
		// Included for a hot index with cold table hydration.
		// The tenant/rank index should be compact, but fetching bodies by row id should expose table-page misses.
		name: "hot-index-cold-table",
		category: "read",
		sizeClass: "cache-overflow",
		description: "Hot secondary-index selection followed by cold table-row hydration.",
	},
	{
		// Included to test composite primary-key storage without normal rowid table layout.
		// Access follows primary-key B-tree order, but physical page order may diverge after splits, so read-ahead should grow only on directional VFS misses.
		name: "ledger-without-rowid-range",
		category: "read",
		sizeClass: "large",
		description: "WITHOUT ROWID composite-primary-key range read.",
	},
	{
		// Included to measure the write/commit path after opening a non-empty database.
		// Schema and root pages should be warm from preload; dirty-page writes and commit transport should dominate rather than read-ahead.
		name: "write-batch-after-wake",
		category: "write",
		sizeClass: "medium",
		description: "Post-wake transactional insert batch into an existing database.",
	},
	{
		// Included to model repeated updates to a tenant/shard subset after wake.
		// Protected cache should help root/index and hot partition pages survive scan churn while commit cost stays visible.
		name: "update-hot-partition",
		category: "write",
		sizeClass: "medium",
		description: "Post-wake indexed update of a hot partition.",
	},
	{
		// Included for storage churn without adding VACUUM as a dominant benchmark.
		// Deletes create free-list/layout churn, then the range read shows whether scan behavior stays healthy.
		name: "delete-churn-range-read",
		category: "write",
		sizeClass: "medium",
		description: "Delete a hot shard range, then scan the remaining rowid table.",
	},
	{
		// Included because CREATE INDEX over existing data scans the source table and writes new index B-trees.
		// Read-ahead may help source reads, but commit/write amplification should remain a major cost.
		name: "migration-create-indexes-large",
		category: "migration",
		sizeClass: "large",
		description: "Schema migration that creates multiple indexes on an existing large table.",
	},
	{
		// Included because skew changes index fanout/cardinality while still requiring a table scan.
		// The source read path should resemble index creation, while index B-tree writes may differ from high-cardinality data.
		name: "migration-create-indexes-skewed-large",
		category: "migration",
		sizeClass: "large",
		description: "Schema migration that creates indexes over skewed existing data.",
	},
	{
		// Included for SQLite migrations that must rebuild a table, such as drop-column or type-change patterns.
		// This should read and rewrite every row, so cache/preload helps less than storage read/write throughput.
		name: "migration-table-rebuild-large",
		category: "migration",
		sizeClass: "large",
		description: "Large table-rebuild migration using create-copy-drop-rename.",
	},
	{
		// Included as the large-data control for schema-only ADD COLUMN migrations.
		// SQLite should update schema metadata without rewriting existing rows.
		name: "migration-add-column-large",
		category: "migration",
		sizeClass: "large",
		description: "Large-table ADD COLUMN migration that should avoid row rewrite.",
	},
	{
		// Included as a low-data migration canary.
		// This should be dominated by schema/root page access and tiny commits, so startup preload should keep it fast.
		name: "migration-ddl-small",
		category: "canary",
		sizeClass: "none",
		description: "Small schema-only migration with CREATE TABLE, ALTER TABLE, and CREATE INDEX.",
	},
];

function usage(exitCode = 1): never {
	console.error(`Usage:
  pnpm --filter kitchen-sink exec tsx scripts/sqlite-realworld-bench.ts [options]

Options:
  --endpoint <url>              Rivet endpoint. Default: ${DEFAULT_ENDPOINT}
  --key <key>                   Actor key suffix. Defaults to a generated key.
  --profile <name>              standard or smoke. Default: standard.
  --only <names>                Comma-separated workload names.
  --small-bytes <n>             Small workload payload bytes.
  --medium-bytes <n>            Medium workload payload bytes.
  --cache-fit-bytes <n>         Cache-fit workload payload bytes.
  --cache-overflow-bytes <n>    Just-over-cache workload payload bytes.
  --large-bytes <n>             Large workload payload bytes.
  --row-bytes <n>               Payload bytes per seeded row. Default: ${DEFAULT_ROW_BYTES}
  --wake-delay-ms <n>           Delay after c.sleep() before the measured main phase. Default: ${DEFAULT_WAKE_DELAY_MS}
  --post-setup-wait-ms <n>      Optional wait after setup before sleep. Default: ${DEFAULT_POST_SETUP_WAIT_MS}
  --output-dir <path>           Results directory. Default: ${DEFAULT_RESULTS_ROOT}/<timestamp>
  --metrics-token <token>       Bearer token for actor /metrics. Default: env or dev-metrics.
  --disable-metadata-lookup     Treat --endpoint as the direct engine endpoint.
  --start-local-envoy           Start this registry's local envoy before driving it.
  --no-start-local-envoy        Use an already-running endpoint.
  --disable-storage-compaction  Start the local engine with storage compaction disabled.
  --disable-sqlite-optimizations
                                Disable all env-gated SQLite/VFS optimizations for baseline runs.

Profiles:
  standard: small=4 MiB, medium=64 MiB, cache-fit=128 MiB, cache-overflow=201 MiB, large=256 MiB.
  smoke:    small=256 KiB, medium=1 MiB, cache-fit=1 MiB, cache-overflow=2 MiB, large=2 MiB.

Workloads:
  ${WORKLOADS.join(", ")}`);
	process.exit(exitCode);
}

function readFlag(argv: string[], name: string): string | undefined {
	const prefix = `${name}=`;
	const inline = argv.find((arg) => arg.startsWith(prefix));
	if (inline) return inline.slice(prefix.length);
	const index = argv.indexOf(name);
	if (index >= 0) return argv[index + 1];
	return undefined;
}

function readNumber(
	argv: string[],
	flag: string,
	envName: string,
	defaultValue: number,
): number {
	const raw = readFlag(argv, flag) ?? process.env[envName];
	if (raw === undefined) return defaultValue;
	const value = Number.parseInt(raw, 10);
	if (!Number.isFinite(value) || value < 0) {
		throw new Error(`${flag} must be a non-negative integer`);
	}
	return value;
}

function parseProfile(value: string | undefined): Profile {
	if (value === undefined || value === "standard") return "standard";
	if (value === "smoke") return "smoke";
	throw new Error("--profile must be standard or smoke");
}

function parseOnly(value: string | undefined): WorkloadName[] {
	if (!value) return [...WORKLOADS];
	const names = value
		.split(",")
		.map((name) => name.trim())
		.filter(Boolean);
	for (const name of names) {
		if (!(WORKLOADS as readonly string[]).includes(name)) {
			throw new Error(`unknown workload in --only: ${name}`);
		}
	}
	return names as WorkloadName[];
}

function timestampForPath(date = new Date()): string {
	return date.toISOString().replace(/[:.]/g, "-");
}

function parseArgs(argv: string[]): Args {
	if (argv.includes("--help") || argv.includes("-h")) usage(0);
	const endpoint = readFlag(argv, "--endpoint") ?? process.env.RIVET_ENDPOINT ?? DEFAULT_ENDPOINT;
	const profile = parseProfile(readFlag(argv, "--profile"));
	const defaultSmallBytes = profile === "smoke" ? 256 * 1024 : 4 * 1024 * 1024;
	const defaultMediumBytes = profile === "smoke" ? 1024 * 1024 : 64 * 1024 * 1024;
	const defaultCacheFitBytes = profile === "smoke" ? 1024 * 1024 : 128 * 1024 * 1024;
	const defaultCacheOverflowBytes =
		profile === "smoke" ? 2 * 1024 * 1024 : 201 * 1024 * 1024;
	const defaultLargeBytes = profile === "smoke" ? 2 * 1024 * 1024 : 256 * 1024 * 1024;
	const shouldStartLocalEnvoy =
		argv.includes("--start-local-envoy") ||
		(!argv.includes("--no-start-local-envoy") &&
			endpoint === DEFAULT_ENDPOINT &&
			process.env.RIVET_ENDPOINT === undefined);
	const outputDir =
		readFlag(argv, "--output-dir") ??
		join(DEFAULT_RESULTS_ROOT, timestampForPath());

	return {
		endpoint,
		key:
			readFlag(argv, "--key") ??
			`sqlite-realworld-${Date.now()}-${crypto.randomUUID().slice(0, 8)}`,
		profile,
		only: parseOnly(readFlag(argv, "--only")),
		smallBytes: readNumber(
			argv,
			"--small-bytes",
			"SQLITE_REALWORLD_SMALL_BYTES",
			defaultSmallBytes,
		),
		mediumBytes: readNumber(
			argv,
			"--medium-bytes",
			"SQLITE_REALWORLD_MEDIUM_BYTES",
			defaultMediumBytes,
		),
		cacheFitBytes: readNumber(
			argv,
			"--cache-fit-bytes",
			"SQLITE_REALWORLD_CACHE_FIT_BYTES",
			defaultCacheFitBytes,
		),
		cacheOverflowBytes: readNumber(
			argv,
			"--cache-overflow-bytes",
			"SQLITE_REALWORLD_CACHE_OVERFLOW_BYTES",
			defaultCacheOverflowBytes,
		),
		largeBytes: readNumber(
			argv,
			"--large-bytes",
			"SQLITE_REALWORLD_LARGE_BYTES",
			defaultLargeBytes,
		),
		rowBytes: readNumber(
			argv,
			"--row-bytes",
			"SQLITE_REALWORLD_ROW_BYTES",
			DEFAULT_ROW_BYTES,
		),
		wakeDelayMs: readNumber(
			argv,
			"--wake-delay-ms",
			"SQLITE_REALWORLD_WAKE_DELAY_MS",
			DEFAULT_WAKE_DELAY_MS,
		),
		postSetupWaitMs: readNumber(
			argv,
			"--post-setup-wait-ms",
			"SQLITE_REALWORLD_POST_SETUP_WAIT_MS",
			DEFAULT_POST_SETUP_WAIT_MS,
		),
		outputDir,
		metricsToken:
			readFlag(argv, "--metrics-token") ??
			process.env.SQLITE_REALWORLD_METRICS_TOKEN ??
			process.env._RIVET_METRICS_TOKEN ??
			"dev-metrics",
		disableMetadataLookup: argv.includes("--disable-metadata-lookup"),
		startLocalEnvoy: shouldStartLocalEnvoy,
		disableStorageCompaction: argv.includes("--disable-storage-compaction"),
		disableSqliteOptimizations: argv.includes("--disable-sqlite-optimizations"),
	};
}

function disabledSqliteOptimizationEnv(): Record<string, string> {
	const env: Record<string, string> = {};
	for (const name of SQLITE_OPT_MODE_ENVS) {
		env[name] = "off";
	}
	for (const name of SQLITE_OPT_BOOLEAN_ENVS) {
		env[name] = "false";
	}
	for (const name of SQLITE_OPT_NUMERIC_ENVS) {
		env[name] = "0";
	}
	return env;
}

function applyDisabledSqliteOptimizations(target: NodeJS.ProcessEnv): void {
	Object.assign(target, disabledSqliteOptimizationEnv());
}

function sqliteOptimizationEnvSnapshot(): Record<string, string | null> {
	const snapshot: Record<string, string | null> = {};
	for (const name of [...SQLITE_OPT_MODE_ENVS, ...SQLITE_OPT_BOOLEAN_ENVS, ...SQLITE_OPT_NUMERIC_ENVS]) {
		snapshot[name] = process.env[name] ?? null;
	}
	return snapshot;
}

function envNumberOrDefault(name: string, defaultValue: number): number {
	const raw = process.env[name];
	if (raw === undefined) return defaultValue;
	const value = Number.parseInt(raw, 10);
	return Number.isFinite(value) ? value : defaultValue;
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function fmtMs(ms: number): string {
	return `${ms.toFixed(1)}ms`;
}

function fmtBytes(bytes: number): string {
	const mib = bytes / 1024 / 1024;
	return `${mib.toFixed(2)} MiB`;
}

function targetBytesFor(args: Args, sizeClass: SizeClass): number {
	switch (sizeClass) {
		case "none":
			return 0;
		case "small":
			return args.smallBytes;
		case "medium":
			return args.mediumBytes;
		case "cache-fit":
			return args.cacheFitBytes;
		case "cache-overflow":
			return args.cacheOverflowBytes;
		case "large":
			return args.largeBytes;
	}
}

function resolveEngineBinary(): string {
	if (process.env.RIVET_ENGINE_BINARY) return process.env.RIVET_ENGINE_BINARY;
	if (existsSync(REPO_ENGINE_BINARY)) return REPO_ENGINE_BINARY;
	throw new Error(
		`No local rivet-engine binary found. Build one with cargo build -p rivet-engine or set RIVET_ENGINE_BINARY.`,
	);
}

function tailEngineLogs(engine: LocalEngine | undefined): string {
	if (!engine) return "";
	const text = engine.logs.join("");
	const lines = text.trimEnd().split("\n");
	return lines.slice(-120).join("\n");
}

async function waitForEngineReady(
	child: ChildProcess,
	endpoint: string,
	logs: string[],
): Promise<void> {
	const deadline = Date.now() + 15_000;
	let lastError: unknown;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`rivet-engine exited before health check passed:\n${logs.join("")}`,
			);
		}

		try {
			const response = await fetch(`${endpoint.replace(/\/$/, "")}/health`);
			if (response.ok) return;
			lastError = new Error(`health returned ${response.status}`);
		} catch (err) {
			lastError = err;
		}

		await sleep(100);
	}

	throw lastError instanceof Error
		? lastError
		: new Error("timed out waiting for rivet-engine");
}

async function startLocalEngine(args: Args): Promise<LocalEngine> {
	const logs: string[] = [];
	const dbRoot = mkdtempSync(join(tmpdir(), "sqlite-realworld-engine-"));
	const metricsPort = await findOpenPort();
	const engineEndpoint = new URL(args.endpoint);
	const guardPort = Number.parseInt(engineEndpoint.port, 10);
	if (!Number.isFinite(guardPort) || guardPort <= 0) {
		throw new Error(`endpoint must include a numeric port: ${args.endpoint}`);
	}
	const guardHost = engineEndpoint.hostname || "127.0.0.1";
	const env = {
		...process.env,
		RIVET__GUARD__HOST: guardHost,
		RIVET__GUARD__PORT: guardPort.toString(),
		RIVET__API_PEER__HOST: guardHost,
		RIVET__API_PEER__PORT: (guardPort + 1).toString(),
		RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
		RIVET__METRICS__HOST: "127.0.0.1",
		RIVET__METRICS__PORT: metricsPort.toString(),
		_RIVET_METRICS_TOKEN: args.metricsToken,
	};
	if (args.disableStorageCompaction) {
		env.RIVET_SQLITE_DISABLE_COMPACTION =
			process.env.RIVET_SQLITE_DISABLE_COMPACTION ?? "1";
	}
	const child = spawn(resolveEngineBinary(), ["start"], {
		env,
		stdio: ["ignore", "pipe", "pipe"],
	});
	child.stdout?.on("data", (chunk) => logs.push(chunk.toString()));
	child.stderr?.on("data", (chunk) => logs.push(chunk.toString()));
	try {
		await waitForEngineReady(child, args.endpoint, logs);
		return { child, dbRoot, logs };
	} catch (err) {
		await stopLocalEngine({ child, dbRoot, logs });
		throw err;
	}
}

async function findOpenPort(): Promise<number> {
	return new Promise((resolvePort, reject) => {
		const server = createServer();
		server.on("error", reject);
		server.listen(0, "127.0.0.1", () => {
			const address = server.address();
			if (address === null || typeof address === "string") {
				server.close(() => reject(new Error("failed to allocate metrics port")));
				return;
			}
			const port = address.port;
			server.close(() => resolvePort(port));
		});
	});
}

async function stopLocalEngine(engine: LocalEngine | undefined): Promise<void> {
	if (!engine) return;
	const { child, dbRoot } = engine;
	if (child.exitCode === null) {
		child.kill("SIGTERM");
		await Promise.race([
			new Promise<void>((resolve) => child.once("exit", () => resolve())),
			sleep(5_000),
		]);
		if (child.exitCode === null) child.kill("SIGKILL");
	}
	rmSync(dbRoot, { recursive: true, force: true });
}

async function waitForRegistryReady(endpoint: string): Promise<void> {
	const deadline = Date.now() + 15_000;
	let lastError: unknown;

	while (Date.now() < deadline) {
		try {
			const response = await fetch(`${endpoint.replace(/\/$/, "")}/metadata`);
			if (response.ok) return;
			lastError = new Error(`metadata returned ${response.status}`);
		} catch (err) {
			lastError = err;
		}

		await sleep(100);
	}

	throw lastError instanceof Error
		? lastError
		: new Error("timed out waiting for local registry");
}

async function configureLocalRunner(endpoint: string): Promise<void> {
	const base = endpoint.replace(/\/$/, "");
	const datacentersResponse = await fetch(`${base}/datacenters?namespace=default`, {
		headers: { Authorization: "Bearer dev" },
	});
	if (!datacentersResponse.ok) {
		throw new Error(
			`failed to list local datacenters: ${datacentersResponse.status} ${await datacentersResponse.text()}`,
		);
	}

	const datacentersBody = (await datacentersResponse.json()) as {
		datacenters: Array<{ name: string }>;
	};
	const datacenter = datacentersBody.datacenters[0]?.name;
	if (!datacenter) throw new Error("local engine returned no datacenters");

	const response = await fetch(`${base}/runner-configs/default?namespace=default`, {
		method: "PUT",
		headers: {
			Authorization: "Bearer dev",
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			datacenters: {
				[datacenter]: {
					normal: {},
				},
			},
		}),
	});
	if (!response.ok) {
		throw new Error(
			`failed to configure local default runner: ${response.status} ${await response.text()}`,
		);
	}
}

async function waitForEnvoy(endpoint: string): Promise<void> {
	const base = endpoint.replace(/\/$/, "");
	const deadline = Date.now() + 15_000;

	while (Date.now() < deadline) {
		const response = await fetch(`${base}/envoys?namespace=default&name=default`, {
			headers: { Authorization: "Bearer dev" },
		});
		if (response.ok) {
			const body = (await response.json()) as {
				envoys: Array<{ envoy_key: string }>;
			};
			if (body.envoys.length > 0) return;
		}

		await sleep(100);
	}

	throw new Error("timed out waiting for local envoy registration");
}

async function retryTransient<T>(
	label: string,
	fn: () => Promise<T>,
	attempts = 3,
): Promise<T> {
	let lastError: unknown;
	for (let attempt = 1; attempt <= attempts; attempt += 1) {
		try {
			return await fn();
		} catch (err) {
			lastError = err;
			const message = err instanceof Error ? err.message : String(err);
			const transient =
				message.includes("timed out") ||
				message.includes("fetch failed") ||
				message.includes("Connection reset") ||
				message.includes("Service unavailable");
			if (!transient || attempt === attempts) break;
			console.warn(`  ${label} failed transiently, retrying ${attempt + 1}/${attempts}`);
			await sleep(1000 * attempt);
		}
	}
	throw lastError;
}

function parsePrometheusLabels(raw: string | undefined): Record<string, string> {
	if (!raw) return {};
	const labels: Record<string, string> = {};
	for (const part of raw.slice(1, -1).split(",")) {
		const separator = part.indexOf("=");
		if (separator < 0) continue;
		const key = part.slice(0, separator);
		const value = part.slice(separator + 1).replace(/^"|"$/g, "");
		labels[key] = value;
	}
	return labels;
}

function metricValue(text: string, name: string): number {
	for (const line of text.split("\n")) {
		if (line.length === 0 || line.startsWith("#")) continue;
		const [series, value] = line.trim().split(/\s+/, 2);
		if (!series || value === undefined) continue;
		const match = /^([^{]+)(\{.*\})?$/.exec(series);
		if (!match || match[1] !== name) continue;
		parsePrometheusLabels(match[2]);
		return Number.parseFloat(value);
	}
	return 0;
}

async function scrapeVfsMetrics(
	endpoint: string,
	actorId: string,
	metricsToken: string,
): Promise<VfsMetricSnapshot> {
	const base = endpoint.replace(/\/$/, "");
	const gatewayToken = process.env.RIVET_TOKEN
		? `@${encodeURIComponent(process.env.RIVET_TOKEN)}`
		: "";
	const response = await fetch(
		`${base}/gateway/${encodeURIComponent(actorId)}${gatewayToken}/metrics`,
		{
			headers: {
				Authorization: `Bearer ${metricsToken}`,
			},
		},
	);
	if (!response.ok) {
		throw new Error(
			`failed to scrape actor metrics: ${response.status} ${await response.text()}`,
		);
	}
	const text = await response.text();
	return {
		resolvePagesTotal: metricValue(text, "sqlite_vfs_resolve_pages_total"),
		resolvePagesRequestedTotal: metricValue(
			text,
			"sqlite_vfs_resolve_pages_requested_total",
		),
		resolvePagesCacheHitsTotal: metricValue(
			text,
			"sqlite_vfs_resolve_pages_cache_hits_total",
		),
		resolvePagesCacheMissesTotal: metricValue(
			text,
			"sqlite_vfs_resolve_pages_cache_misses_total",
		),
		getPagesTotal: metricValue(text, "sqlite_vfs_get_pages_total"),
		pagesFetchedTotal: metricValue(text, "sqlite_vfs_pages_fetched_total"),
		prefetchPagesTotal: metricValue(text, "sqlite_vfs_prefetch_pages_total"),
		bytesFetchedTotal: metricValue(text, "sqlite_vfs_bytes_fetched_total"),
		prefetchBytesTotal: metricValue(text, "sqlite_vfs_prefetch_bytes_total"),
		getPagesDurationSecondsSum: metricValue(
			text,
			"sqlite_vfs_get_pages_duration_seconds_sum",
		),
		getPagesDurationSecondsCount: metricValue(
			text,
			"sqlite_vfs_get_pages_duration_seconds_count",
		),
	};
}

function diffVfsMetrics(
	after: VfsMetricSnapshot,
	before: VfsMetricSnapshot,
): VfsMetricSnapshot {
	return Object.fromEntries(
		Object.keys(after).map((key) => [
			key,
			after[key as keyof VfsMetricSnapshot] -
				before[key as keyof VfsMetricSnapshot],
		]),
	) as unknown as VfsMetricSnapshot;
}

function emptyVfsMetrics(): VfsMetricSnapshot {
	return {
		resolvePagesTotal: 0,
		resolvePagesRequestedTotal: 0,
		resolvePagesCacheHitsTotal: 0,
		resolvePagesCacheMissesTotal: 0,
		getPagesTotal: 0,
		pagesFetchedTotal: 0,
		prefetchPagesTotal: 0,
		bytesFetchedTotal: 0,
		prefetchBytesTotal: 0,
		getPagesDurationSecondsSum: 0,
		getPagesDurationSecondsCount: 0,
	};
}

function writeResults(outputDir: string, document: unknown): void {
	mkdirSync(outputDir, { recursive: true });
	writeFileSync(
		join(outputDir, "results.json"),
		`${JSON.stringify(document, null, "\t")}\n`,
	);
}

function writeSummary(outputDir: string, results: BenchmarkResult[]): void {
	const lines = [
		"SQLite real-world benchmark",
		"",
		"Server SQLite time only. Setup time, sleep delay, wake/cold-start time, and client RTT are not included.",
		"",
		"| workload | category | size | server_ms | get_pages | fetched_pages | cache_hits | cache_misses | rows/ops | pages |",
		"| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
	];
	for (const result of results) {
		const rowsOrOps =
			typeof result.main.rows === "number"
				? result.main.rows
				: typeof result.main.ops === "number"
					? result.main.ops
					: "";
		lines.push(
			`| ${result.workload} | ${result.category} | ${fmtBytes(result.targetBytes)} | ${result.main.ms.toFixed(1)} | ${result.vfsMetrics.getPagesTotal} | ${result.vfsMetrics.pagesFetchedTotal} | ${result.vfsMetrics.resolvePagesCacheHitsTotal} | ${result.vfsMetrics.resolvePagesCacheMissesTotal} | ${rowsOrOps} | ${result.main.pageCount} |`,
		);
	}
	writeFileSync(join(outputDir, "summary.md"), `${lines.join("\n")}\n`);
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	if (args.disableSqliteOptimizations) {
		applyDisabledSqliteOptimizations(process.env);
	}
	process.env._RIVET_METRICS_TOKEN = args.metricsToken;
	const selectedSpecs = WORKLOAD_SPECS.filter((spec) =>
		args.only.includes(spec.name),
	);
	let engine: LocalEngine | undefined;

	if (args.startLocalEnvoy) {
		process.env.RIVET_ENDPOINT = args.endpoint;
		process.env.RIVET_TOKEN = process.env.RIVET_TOKEN ?? "dev";
		try {
			engine = await startLocalEngine(args);
			await configureLocalRunner(args.endpoint);
			await import("@rivetkit/sql-loader");
			const { registry } = await import("../src/index.ts");
			registry.start();
			await waitForRegistryReady(args.endpoint);
			await waitForEnvoy(args.endpoint);
			await sleep(500);
		} catch (err) {
			await stopLocalEngine(engine);
			throw err;
		}
	}

	const outputDir = resolve(REPO_ROOT, args.outputDir);
	const client = createClient<typeof registry>({
		endpoint: args.endpoint,
		disableMetadataLookup: args.disableMetadataLookup,
	});
	type BenchHandle = ReturnType<typeof client.sqliteRealworldBench.getOrCreate>;
	const results: BenchmarkResult[] = [];
	const startedAt = new Date().toISOString();

	const startupPreloadMaxBytes = envNumberOrDefault(
		"RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_MAX_BYTES",
		DEFAULT_STARTUP_PRELOAD_MAX_BYTES,
	);
	const vfsPageCacheCapacityPages = envNumberOrDefault(
		"RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_CAPACITY_PAGES",
		DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES,
	);
	const vfsPageCacheBytes = vfsPageCacheCapacityPages * SQLITE_PAGE_SIZE_BYTES;
	const resultDocument = {
		schemaVersion: 1,
		startedAt,
		finishedAt: null as string | null,
		config: {
			endpoint: args.endpoint,
			profile: args.profile,
			selectedWorkloads: selectedSpecs.map((spec) => spec.name),
			sizes: {
				smallBytes: args.smallBytes,
				mediumBytes: args.mediumBytes,
				cacheFitBytes: args.cacheFitBytes,
				cacheOverflowBytes: args.cacheOverflowBytes,
				largeBytes: args.largeBytes,
				rowBytes: args.rowBytes,
			},
			metricsToken: args.metricsToken,
			wakeDelayMs: args.wakeDelayMs,
			postSetupWaitMs: args.postSetupWaitMs,
			startLocalEnvoy: args.startLocalEnvoy,
			disableStorageCompaction: args.disableStorageCompaction,
			sqliteOptimizationsDisabled: args.disableSqliteOptimizations,
			sqliteOptimizationEnv: sqliteOptimizationEnvSnapshot(),
			cacheSizing: {
				sqlitePageSizeBytes: SQLITE_PAGE_SIZE_BYTES,
				startupPreloadMaxBytes,
				vfsPageCacheCapacityPages,
				vfsPageCacheCapacityBytes: vfsPageCacheBytes,
				largeBytesExceedsConfiguredVfsCache: args.largeBytes > vfsPageCacheBytes,
			},
		},
		cacheConfigProbe: null as CacheConfigResult | null,
		results,
	};

	console.log("SQLite real-world benchmark");
	console.log(`endpoint=${args.endpoint}`);
	console.log(`profile=${args.profile}`);
	console.log(`output=${outputDir}`);
	console.log(
		`sqlite_optimizations=${args.disableSqliteOptimizations ? "disabled" : "default"}`,
	);
	console.log(
		`cache_fit=${fmtBytes(args.cacheFitBytes)} cache_overflow=${fmtBytes(args.cacheOverflowBytes)} large=${fmtBytes(args.largeBytes)} vfs_cache_configured=${fmtBytes(vfsPageCacheBytes)} startup_preload_configured=${fmtBytes(startupPreloadMaxBytes)}`,
	);
	console.log("server SQLite time only; setup, sleep, wake, and RTT are excluded");

	try {
		mkdirSync(outputDir, { recursive: true });

		const probeKey = ["sqlite-realworld-bench", args.key, "cache-config"];
		const probeHandle = client.sqliteRealworldBench.getOrCreate(probeKey);
		resultDocument.cacheConfigProbe = (await retryTransient(
			"cache config probe",
			() => probeHandle.inspectCacheConfig(),
		)) as CacheConfigResult;
		await probeHandle.goToSleep();
		writeResults(outputDir, resultDocument);

		for (const spec of selectedSpecs) {
			const targetBytes = targetBytesFor(args, spec.sizeClass);
			const actorKey = ["sqlite-realworld-bench", args.key, spec.name];
			const handle: BenchHandle =
				client.sqliteRealworldBench.getOrCreate(actorKey);

			console.log(`\n${spec.name}`);
			console.log(`  ${spec.description}`);
			console.log(`  actor_key=${actorKey.join("/")}`);
			console.log(`  target=${fmtBytes(targetBytes)}`);
			const actorId = await retryTransient("actor resolve", () => handle.resolve());
			console.log(`  actor_id=${actorId}`);

			let setup: SetupResult | null = null;
			if (spec.sizeClass !== "none") {
				console.log("  setup...");
				setup = (await handle.setupWorkload({
					workload: spec.name,
					targetBytes,
					rowBytes: args.rowBytes,
				})) as SetupResult;
				console.log(
					`  setup rows=${setup.rows} pages=${setup.pageCount} setup_ms=${fmtMs(setup.setupMs)}`,
				);
			} else {
				console.log("  setup skipped");
				setup = (await handle.setupWorkload({
					workload: spec.name,
					targetBytes: 0,
					rowBytes: args.rowBytes,
				})) as SetupResult;
			}

			if (args.postSetupWaitMs > 0) await sleep(args.postSetupWaitMs);

			console.log("  sleep...");
			await handle.goToSleep();
			await sleep(args.wakeDelayMs);

			console.log("  cold-wake main phase...");
			const coldHandle = client.sqliteRealworldBench.getOrCreate(actorKey);
			const mainResult = (await retryTransient("main workload", () =>
				coldHandle.runWorkload({
					workload: spec.name,
					targetBytes,
				}),
			)) as MainResult;
			const afterMainMetrics = await scrapeVfsMetrics(
				args.endpoint,
				actorId,
				args.metricsToken,
			);
			const vfsMetrics = diffVfsMetrics(afterMainMetrics, emptyVfsMetrics());
			console.log(
				`  server=${fmtMs(mainResult.ms)} pages=${mainResult.pageCount} get_pages=${vfsMetrics.getPagesTotal} fetched_pages=${vfsMetrics.pagesFetchedTotal}`,
			);

			results.push({
				workload: spec.name,
				description: spec.description,
				category: spec.category,
				sizeClass: spec.sizeClass,
				targetBytes,
				actorKey,
				actorId,
				setup,
				main: mainResult,
				vfsMetrics,
			});
			writeResults(outputDir, resultDocument);
			writeSummary(outputDir, results);
		}

		resultDocument.finishedAt = new Date().toISOString();
		writeResults(outputDir, resultDocument);
		writeSummary(outputDir, results);

		console.log("\nResults");
		for (const result of results) {
			console.log(
				`  ${result.workload}: server=${fmtMs(result.main.ms)} size=${fmtBytes(result.targetBytes)} get_pages=${result.vfsMetrics.getPagesTotal} fetched_pages=${result.vfsMetrics.pagesFetchedTotal}`,
			);
		}
		console.log(`\nwrote ${join(outputDir, "results.json")}`);
		console.log(`wrote ${join(outputDir, "summary.md")}`);
	} catch (err) {
		const engineLogs = tailEngineLogs(engine);
		if (engineLogs) {
			console.error("\nengine log tail:");
			console.error(engineLogs);
		}
		throw err;
	} finally {
		await client.dispose().catch(() => undefined);
		await stopLocalEngine(engine);
	}
}

main()
	.then(() => {
		process.exit(0);
	})
	.catch((err: unknown) => {
		const message = err instanceof Error ? err.stack ?? err.message : String(err);
		console.error(message);
		process.exit(1);
	});
