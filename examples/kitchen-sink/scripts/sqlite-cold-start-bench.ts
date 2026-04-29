#!/usr/bin/env -S pnpm exec tsx

import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

interface Args {
	endpoint: string;
	key: string;
	targetBytes: number;
	rowBytes: number;
	batchRows: number;
	transactionBytes: number;
	wakeDelayMs: number;
	metricsToken: string;
	disableMetadataLookup: boolean;
	startLocalEnvoy: boolean;
}

interface WriteResult {
	ms: number;
	writeWallMs: number;
	randomStringMs: number;
	sqliteInsertMs: number;
	commitMs: number;
	ops: number;
	rows: number;
	transactions: number;
	bytes: number;
	rowBytes: number;
	batchRows: number;
	transactionBytes: number;
}

interface ReadResult {
	ms: number;
	ops: number;
	rows: number;
	bytes: number;
	expectedBytes: number;
	probeMatches: number;
}

interface LocalEngine {
	child: ChildProcess;
	dbRoot: string;
	logs: string[];
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
	commitTotal: number;
	commitDurationSecondsTotal: number;
	commitRequestBuildSecondsTotal: number;
	commitSerializeSecondsTotal: number;
	commitTransportSecondsTotal: number;
	commitStateUpdateSecondsTotal: number;
}

const DEFAULT_ENDPOINT = "http://127.0.0.1:6420";
const DEFAULT_TARGET_BYTES = 50 * 1024 * 1024;
const DEFAULT_ROW_BYTES = 16 * 1024;
const DEFAULT_BATCH_ROWS = 8;
const DEFAULT_TRANSACTION_BYTES = 64 * 1024;
const DEFAULT_WAKE_DELAY_MS = 2000;
const REPO_ENGINE_BINARY = fileURLToPath(
	new URL("../../../target/debug/rivet-engine", import.meta.url),
);

function usage(): never {
	console.error(`Usage:
  pnpm --filter kitchen-sink exec tsx scripts/sqlite-cold-start-bench.ts [options]

Options:
  --endpoint <url>              Rivet endpoint. Default: ${DEFAULT_ENDPOINT}
  --key <key>                   Actor key suffix. Defaults to a generated key.
  --bytes <n>                   Total bytes to write and read. Default: ${DEFAULT_TARGET_BYTES}
  --row-bytes <n>               Bytes per random string row. Default: ${DEFAULT_ROW_BYTES}
  --batch-rows <n>              Rows per INSERT statement. Default: ${DEFAULT_BATCH_ROWS}
	  --transaction-bytes <n>       Bytes per SQLite transaction. Default: ${DEFAULT_TRANSACTION_BYTES}
	  --wake-delay-ms <n>           Delay after c.sleep() before the cold read. Default: ${DEFAULT_WAKE_DELAY_MS}
	  --metrics-token <token>       Bearer token for actor /metrics. Default: env or dev-metrics.
	  --disable-metadata-lookup     Treat --endpoint as the direct engine endpoint.
	  --start-local-envoy           Start this registry's local envoy before driving it.
  --no-start-local-envoy        Use an already-running endpoint.

Environment:
	  RIVET_ENDPOINT, SQLITE_COLD_START_BYTES, SQLITE_COLD_START_ROW_BYTES,
	  SQLITE_COLD_START_BATCH_ROWS, SQLITE_COLD_START_TRANSACTION_BYTES,
	  SQLITE_COLD_START_WAKE_DELAY_MS, SQLITE_COLD_START_METRICS_TOKEN,
	  _RIVET_METRICS_TOKEN`);
	process.exit(1);
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
	if (!Number.isFinite(value) || value < 1) {
		throw new Error(`${flag} must be a positive integer`);
	}
	return value;
}

function parseArgs(argv: string[]): Args {
	if (argv.includes("--help") || argv.includes("-h")) usage();
	const endpoint = readFlag(argv, "--endpoint") ?? process.env.RIVET_ENDPOINT ?? DEFAULT_ENDPOINT;
	const shouldStartLocalEnvoy =
		argv.includes("--start-local-envoy") ||
		(!argv.includes("--no-start-local-envoy") &&
			endpoint === DEFAULT_ENDPOINT &&
			process.env.RIVET_ENDPOINT === undefined);

	return {
		endpoint,
		key:
			readFlag(argv, "--key") ??
			`sqlite-cold-start-${Date.now()}-${crypto.randomUUID().slice(0, 8)}`,
		targetBytes: readNumber(
			argv,
			"--bytes",
			"SQLITE_COLD_START_BYTES",
			DEFAULT_TARGET_BYTES,
		),
		rowBytes: readNumber(
			argv,
			"--row-bytes",
			"SQLITE_COLD_START_ROW_BYTES",
			DEFAULT_ROW_BYTES,
		),
		batchRows: readNumber(
			argv,
			"--batch-rows",
			"SQLITE_COLD_START_BATCH_ROWS",
			DEFAULT_BATCH_ROWS,
		),
		transactionBytes: readNumber(
			argv,
			"--transaction-bytes",
			"SQLITE_COLD_START_TRANSACTION_BYTES",
			DEFAULT_TRANSACTION_BYTES,
		),
			wakeDelayMs: readNumber(
				argv,
				"--wake-delay-ms",
				"SQLITE_COLD_START_WAKE_DELAY_MS",
				DEFAULT_WAKE_DELAY_MS,
			),
			metricsToken:
				readFlag(argv, "--metrics-token") ??
				process.env.SQLITE_COLD_START_METRICS_TOKEN ??
				process.env._RIVET_METRICS_TOKEN ??
				"dev-metrics",
			disableMetadataLookup: argv.includes("--disable-metadata-lookup"),
			startLocalEnvoy: shouldStartLocalEnvoy,
		};
	}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function timed<T>(fn: () => Promise<T>): Promise<{ result: T; ms: number }> {
	const start = performance.now();
	const result = await fn();
	return { result, ms: performance.now() - start };
}

function fmtMs(ms: number): string {
	return `${ms.toFixed(1)}ms`;
}

function fmtBytes(bytes: number): string {
	const mib = bytes / 1024 / 1024;
	return `${mib.toFixed(2)} MiB`;
}

function fmtCount(value: number): string {
	return Number.isInteger(value) ? value.toString() : value.toFixed(3);
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

function metricValue(
	text: string,
	name: string,
	matchLabels: Record<string, string> = {},
): number {
	for (const line of text.split("\n")) {
		if (line.length === 0 || line.startsWith("#")) continue;
		const [series, value] = line.trim().split(/\s+/, 2);
		if (!series || value === undefined) continue;
		const match = /^([^{]+)(\{.*\})?$/.exec(series);
		if (!match || match[1] !== name) continue;
		const labels = parsePrometheusLabels(match[2]);
		let matches = true;
		for (const [key, expected] of Object.entries(matchLabels)) {
			if (labels[key] !== expected) {
				matches = false;
				break;
			}
		}
		if (matches) return Number.parseFloat(value);
	}
	return 0;
}

async function scrapeMetrics(
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
		commitTotal: metricValue(text, "sqlite_vfs_commit_total"),
		commitDurationSecondsTotal: metricValue(
			text,
			"sqlite_vfs_commit_duration_seconds_total",
			{ phase: "total" },
		),
		commitRequestBuildSecondsTotal: metricValue(
			text,
			"sqlite_vfs_commit_phase_duration_seconds_total",
			{ phase: "request_build" },
		),
		commitSerializeSecondsTotal: metricValue(
			text,
			"sqlite_vfs_commit_phase_duration_seconds_total",
			{ phase: "serialize" },
		),
		commitTransportSecondsTotal: metricValue(
			text,
			"sqlite_vfs_commit_phase_duration_seconds_total",
			{ phase: "transport" },
		),
		commitStateUpdateSecondsTotal: metricValue(
			text,
			"sqlite_vfs_commit_phase_duration_seconds_total",
			{ phase: "state_update" },
		),
	};
}

function diffMetrics(
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

function printVfsMetricDelta(label: string, metrics: VfsMetricSnapshot): void {
	console.log(`  ${label} VFS get_pages round trips: ${fmtCount(metrics.getPagesTotal)}`);
	console.log(
		`  ${label} VFS fetched: ${fmtCount(metrics.pagesFetchedTotal)} pages / ${fmtBytes(metrics.bytesFetchedTotal)}`,
	);
	console.log(
		`  ${label} VFS prefetch: ${fmtCount(metrics.prefetchPagesTotal)} pages / ${fmtBytes(metrics.prefetchBytesTotal)}`,
	);
	console.log(
		`  ${label} VFS cache: hits=${fmtCount(metrics.resolvePagesCacheHitsTotal)} misses=${fmtCount(metrics.resolvePagesCacheMissesTotal)} requested=${fmtCount(metrics.resolvePagesRequestedTotal)}`,
	);
	console.log(
		`  ${label} VFS get_pages transport: ${fmtMs(metrics.getPagesDurationSecondsSum * 1000)} over ${fmtCount(metrics.getPagesDurationSecondsCount)} calls`,
	);
}

function assertRead(
	label: string,
	read: ReadResult,
	expectedBytes: number,
	expectedRows: number,
): void {
	if (read.bytes !== expectedBytes || read.expectedBytes !== expectedBytes) {
		throw new Error(
			`${label} read ${read.bytes} bytes, expected ${expectedBytes} bytes`,
		);
	}
	if (read.rows !== expectedRows) {
		throw new Error(`${label} read ${read.rows} rows, expected ${expectedRows}`);
	}
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

async function startLocalEngine(endpoint: string): Promise<LocalEngine> {
	const logs: string[] = [];
	const dbRoot = mkdtempSync(join(tmpdir(), "sqlite-cold-start-engine-"));
		const child = spawn(resolveEngineBinary(), ["start"], {
			env: {
				...process.env,
				RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
				RIVET_SQLITE_DISABLE_COMPACTION:
					process.env.RIVET_SQLITE_DISABLE_COMPACTION ?? "1",
				_RIVET_METRICS_TOKEN:
					process.env._RIVET_METRICS_TOKEN ??
					process.env.SQLITE_COLD_START_METRICS_TOKEN ??
					"dev-metrics",
			},
			stdio: ["ignore", "pipe", "pipe"],
		});
	child.stdout?.on("data", (chunk) => logs.push(chunk.toString()));
	child.stderr?.on("data", (chunk) => logs.push(chunk.toString()));
	try {
		await waitForEngineReady(child, endpoint, logs);
		return { child, dbRoot, logs };
	} catch (err) {
		await stopLocalEngine({ child, dbRoot, logs });
		throw err;
	}
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

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	let engine: LocalEngine | undefined;
	process.env._RIVET_METRICS_TOKEN = args.metricsToken;

	if (args.startLocalEnvoy) {
		engine = await startLocalEngine(args.endpoint);
		await configureLocalRunner(args.endpoint);
		await import("@rivetkit/sql-loader");
		const { registry } = await import("../src/index.ts");
		registry.start();
		await waitForRegistryReady(args.endpoint);
		await waitForEnvoy(args.endpoint);
	}

	const client = createClient<typeof registry>({
		endpoint: args.endpoint,
		disableMetadataLookup: args.disableMetadataLookup,
	});
	const actorKey = ["sqlite-cold-start-bench", args.key];
	const handle = client.sqliteColdStartBench.getOrCreate(actorKey);
	const actorId = await handle.resolve();

	console.log("SQLite cold-start benchmark");
	console.log(`endpoint=${args.endpoint}`);
	console.log(`actor_key=${actorKey.join("/")}`);
	console.log(`actor_id=${actorId}`);
	console.log(`start_local_envoy=${args.startLocalEnvoy}`);
	console.log(
		`target=${fmtBytes(args.targetBytes)} row_bytes=${args.rowBytes} batch_rows=${args.batchRows} transaction_bytes=${args.transactionBytes}`,
	);

	try {
		console.log("\nreset...");
		await handle.reset();

		console.log("write random strings...");
			const write = await timed(() =>
				handle.writeRandomStrings({
					targetBytes: args.targetBytes,
					rowBytes: args.rowBytes,
					batchRows: args.batchRows,
					transactionBytes: args.transactionBytes,
				}),
			);
			const writeResult = write.result as WriteResult;
			const afterWriteMetrics = await scrapeMetrics(
				args.endpoint,
				actorId,
				args.metricsToken,
			);

			console.log("hot read...");
			const hotRead = await timed(() => handle.readAll());
			const hotReadResult = hotRead.result as ReadResult;
			assertRead("hot", hotReadResult, writeResult.bytes, writeResult.rows);
			const afterHotReadMetrics = await scrapeMetrics(
				args.endpoint,
				actorId,
				args.metricsToken,
			);
			const hotReadMetrics = diffMetrics(afterHotReadMetrics, afterWriteMetrics);

			console.log("sleep...");
			await handle.goToSleep();
			await sleep(args.wakeDelayMs);

			console.log("wake read...");
			const coldHandle = client.sqliteColdStartBench.getOrCreate(actorKey);
			const coldRead = await timed(() => coldHandle.readAll());
			const coldReadResult = coldRead.result as ReadResult;
			assertRead("wake", coldReadResult, writeResult.bytes, writeResult.rows);
			const afterWakeReadMetrics = await scrapeMetrics(
				args.endpoint,
				actorId,
				args.metricsToken,
			);

			console.log("\nResults");
		console.log(`  rows: ${writeResult.rows}`);
		console.log(`  transactions: ${writeResult.transactions}`);
		console.log(`  bytes: ${fmtBytes(writeResult.bytes)}`);
		console.log(
			`  insert server: ${fmtMs(writeResult.ms)} (insert=${fmtMs(writeResult.sqliteInsertMs)}, commit=${fmtMs(writeResult.commitMs)}, random_strings=${fmtMs(writeResult.randomStringMs)})`,
		);
		console.log(`  insert e2e: ${fmtMs(write.ms)}`);
		console.log(`  hot read server: ${fmtMs(hotReadResult.ms)}`);
			console.log(`  hot read e2e: ${fmtMs(hotRead.ms)}`);
			console.log(`  wake read server: ${fmtMs(coldReadResult.ms)}`);
			console.log(`  wake read e2e: ${fmtMs(coldRead.ms)}`);
			console.log(
				`  wake overhead estimate: ${fmtMs(Math.max(0, coldRead.ms - coldReadResult.ms))}`,
			);
			printVfsMetricDelta("hot read", hotReadMetrics);
			printVfsMetricDelta("wake read actor-lifetime", afterWakeReadMetrics);
			console.log(
				"  wake read actor-lifetime VFS metrics include startup DB work before the read action.",
			);
		console.log(
			`  probe matches: hot=${hotReadResult.probeMatches} wake=${coldReadResult.probeMatches}`,
		);
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
