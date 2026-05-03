#!/usr/bin/env -S pnpm exec tsx

import { spawn, type ChildProcess } from "node:child_process";
import {
	appendFileSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	readdirSync,
	rmSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const REPO_ROOT = fileURLToPath(new URL("../../..", import.meta.url));
const EXAMPLE_DIR = fileURLToPath(new URL("..", import.meta.url));
const REPO_ENGINE_BINARY = fileURLToPath(
	new URL("../../../target/debug/rivet-engine", import.meta.url),
);
const DEFAULT_ENGINE_PORT = 6520;
const DEFAULT_OUTPUT_DIR = ".agent/benchmarks/sqlite-memory-soak";
const SQLITE_PAGE_SIZE_BYTES = 4096;
const CLOCK_TICKS_PER_SECOND = 100;

interface Args {
	endpoint: string;
	serverPort: number;
	seed: string;
	actors: number;
	cycles: number;
	durationMs: number;
	cycleIntervalMs: number;
	actorStartIntervalMs: number;
	concurrency: number;
	spikeMinConcurrency: number;
	spikeMaxConcurrency: number;
	spikePeriodMs: number;
	insertRows: number;
	rowBytes: number;
	scanRows: number;
	sampleIntervalMs: number;
	wakeEvery: number;
	wakeDelayMs: number;
	churnSleepAfterMs: number;
	sleepLogTimeoutMs: number;
	preWorkloadWaitMs: number;
	postChurnWaitMs: number;
	postCleanupWaitMs: number;
	requestLifespanSeconds: number;
	serverlessMaxStartPayloadBytes: number;
	outputDir: string;
	metricsToken: string;
	reset: boolean;
	cleanup: boolean;
	forceGcSamples: boolean;
	keepStorage: boolean;
}

interface ManagedChild {
	child: ChildProcess;
	label: string;
	logPath: string;
	logs: string[];
}

interface LocalEngine extends ManagedChild {
	dbRoot: string;
}

interface MemorySample {
	kind: "memory_sample";
	runId: string;
	elapsedMs: number;
	timestamp: string;
	harness: ProcMemory;
	engine: ProcMemory;
	kitchenSink: ProcMemory;
	kitchenSinkBreakdown: unknown;
}

interface ProcMemory {
	pid: number | null;
	alive: boolean;
	rssBytes: number | null;
	hwmRssBytes: number | null;
	vmSizeBytes: number | null;
	threads: number | null;
	procState?: string | null;
	cpuUserSeconds?: number | null;
	cpuSystemSeconds?: number | null;
	cpuTotalSeconds?: number | null;
	openFds?: number | null;
	io?: ProcIo;
	smapsRollup?: Record<string, number>;
	error?: string;
}

interface ProcIo {
	readBytes: number | null;
	writeBytes: number | null;
	syscr: number | null;
	syscw: number | null;
}

function kitchenSinkPidFromBreakdown(breakdown: unknown): number | undefined {
	if (typeof breakdown !== "object" || breakdown === null) return undefined;
	const pid = (breakdown as { pid?: unknown }).pid;
	return typeof pid === "number" && Number.isInteger(pid) ? pid : undefined;
}

function usage(exitCode = 1): never {
	console.error(`Usage:
  pnpm --filter kitchen-sink memory-soak [options]

Options:
  --endpoint <url>             Engine endpoint. Default: http://127.0.0.1:6520.
  --server-port <n>            Kitchen-sink HTTP port. Default: open port.
  --seed <seed>                Actor key seed. Default: generated.
  --actors <n>                 Actor instances. Default: 4.
  --cycles <n>                 Max cycles per actor. Default: 20, or unbounded with --duration-ms.
  --duration-ms <n>            Stop scheduling cycles after this duration. Default: 0.
  --cycle-interval-ms <n>      Fixed interval per actor cycle. Default: 1000.
  --actor-start-interval-ms <n>
                               Fixed interval between actor cold starts. Default: 1000.
  --concurrency <n>            Concurrent actor drivers. Default: 4.
  --spike-min-concurrency <n>  If >0, enable spike mode with this minimum target concurrency. Default: 0.
  --spike-max-concurrency <n>  Spike mode maximum target concurrency. Default: --concurrency.
  --spike-period-ms <n>        Full up/down spike period. Default: 60000.
  --insert-rows <n>            Rows inserted per cycle. Default: 128.
  --row-bytes <n>              randomblob bytes per inserted row. Default: 16384.
  --scan-rows <n>              Rows scanned per cycle. Default: 512.
  --sample-interval-ms <n>     Memory sample interval. Default: 1000.
  --wake-every <n>             Sleep each actor every N cycles. Default: 0.
  --wake-delay-ms <n>          Delay after sleep. Default: 1000.
  --churn-sleep-after-ms <n>   If >0, sleep each actor through the engine API after this many ms, then spawn another. Default: 0.
  --sleep-log-timeout-ms <n>   Timeout waiting for the actor sleeping log after API sleep. Default: 10000.
  --pre-workload-wait-ms <n>   Sample idle engine and kitchen-sink before creating actors. Default: 0.
  --post-churn-wait-ms <n>     Sample after churn completes and before cleanup wakes actors. Default: 0.
  --post-cleanup-wait-ms <n>   Final sample window after cleanup. Default: 5000.
  --request-lifespan-seconds <n>
                               Serverless request lifespan. Default: scheduled work plus startup and slow-tail margin.
  --serverless-max-start-payload-bytes <n>
                               Local /api/rivet/start body limit. Default: 8388608.
  --output-dir <path>          Output directory. Default: ${DEFAULT_OUTPUT_DIR}.
  --metrics-token <token>      Engine metrics token. Default: dev-metrics.
  --no-reset                   Reuse actor DBs instead of resetting first.
  --no-cleanup                 Leave actor DB contents after run.
  --force-gc-samples           Request /debug/memory?gc=1 on each sample.
  --keep-storage               Keep the temp engine storage directory.

The harness refuses port 6420 by default so it does not collide with the normal local engine.`);
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
		throw new Error(`invalid ${flag}: ${raw}`);
	}
	return value;
}

function timestampRunPrefix(date = new Date()): string {
	const pad = (value: number) => value.toString().padStart(2, "0");
	return [
		date.getFullYear(),
		pad(date.getMonth() + 1),
		pad(date.getDate()),
		"-",
		pad(date.getHours()),
		pad(date.getMinutes()),
		pad(date.getSeconds()),
	].join("");
}

function scheduledWorkMs(args: Args): number {
	if (args.durationMs > 0) return args.durationMs;
	return args.cycles * args.cycleIntervalMs;
}

function defaultRequestLifespanSeconds(args: Args): number {
	const resetBudgetMs = args.reset ? args.actorStartIntervalMs * args.actors : 0;
	const slowTailBudgetMs = Math.max(5 * 60_000, args.cycleIntervalMs * 2);
	return Math.max(
		300,
		Math.ceil(
			(scheduledWorkMs(args) +
				args.preWorkloadWaitMs +
				args.postCleanupWaitMs +
				args.postChurnWaitMs +
				resetBudgetMs +
				slowTailBudgetMs) /
				1000,
		),
	);
}

function parseArgs(argv: string[]): Args {
	if (argv.includes("--help") || argv.includes("-h")) usage(0);

	const endpoint =
		readFlag(argv, "--endpoint") ??
		process.env.SQLITE_MEMORY_SOAK_ENDPOINT ??
		`http://127.0.0.1:${DEFAULT_ENGINE_PORT}`;
	const endpointUrl = new URL(endpoint);
	if (endpointUrl.port === "6420") {
		throw new Error("sqlite-memory-soak must not run the engine on port 6420");
	}

	const args: Args = {
		endpoint,
		serverPort: readNumber(argv, "--server-port", "SQLITE_MEMORY_SOAK_SERVER_PORT", 0),
		seed:
			readFlag(argv, "--seed") ??
			process.env.SQLITE_MEMORY_SOAK_SEED ??
			`${timestampRunPrefix()}-sqlite-memory-soak`,
		actors: readNumber(argv, "--actors", "SQLITE_MEMORY_SOAK_ACTORS", 4),
		cycles: readNumber(argv, "--cycles", "SQLITE_MEMORY_SOAK_CYCLES", 20),
		durationMs: readNumber(
			argv,
			"--duration-ms",
			"SQLITE_MEMORY_SOAK_DURATION_MS",
			0,
		),
		cycleIntervalMs: readNumber(
			argv,
			"--cycle-interval-ms",
			"SQLITE_MEMORY_SOAK_CYCLE_INTERVAL_MS",
			1000,
		),
		actorStartIntervalMs: readNumber(
			argv,
			"--actor-start-interval-ms",
			"SQLITE_MEMORY_SOAK_ACTOR_START_INTERVAL_MS",
			1000,
		),
		concurrency: readNumber(
			argv,
			"--concurrency",
			"SQLITE_MEMORY_SOAK_CONCURRENCY",
			4,
		),
		spikeMinConcurrency: readNumber(
			argv,
			"--spike-min-concurrency",
			"SQLITE_MEMORY_SOAK_SPIKE_MIN_CONCURRENCY",
			0,
		),
		spikeMaxConcurrency: readNumber(
			argv,
			"--spike-max-concurrency",
			"SQLITE_MEMORY_SOAK_SPIKE_MAX_CONCURRENCY",
			0,
		),
		spikePeriodMs: readNumber(
			argv,
			"--spike-period-ms",
			"SQLITE_MEMORY_SOAK_SPIKE_PERIOD_MS",
			60_000,
		),
		insertRows: readNumber(
			argv,
			"--insert-rows",
			"SQLITE_MEMORY_SOAK_INSERT_ROWS",
			128,
		),
		rowBytes: readNumber(
			argv,
			"--row-bytes",
			"SQLITE_MEMORY_SOAK_ROW_BYTES",
			16 * 1024,
		),
		scanRows: readNumber(
			argv,
			"--scan-rows",
			"SQLITE_MEMORY_SOAK_SCAN_ROWS",
			512,
		),
		sampleIntervalMs: readNumber(
			argv,
			"--sample-interval-ms",
			"SQLITE_MEMORY_SOAK_SAMPLE_INTERVAL_MS",
			1000,
		),
		wakeEvery: readNumber(
			argv,
			"--wake-every",
			"SQLITE_MEMORY_SOAK_WAKE_EVERY",
			0,
		),
		wakeDelayMs: readNumber(
			argv,
			"--wake-delay-ms",
			"SQLITE_MEMORY_SOAK_WAKE_DELAY_MS",
			1000,
		),
		churnSleepAfterMs: readNumber(
			argv,
			"--churn-sleep-after-ms",
			"SQLITE_MEMORY_SOAK_CHURN_SLEEP_AFTER_MS",
			0,
		),
		sleepLogTimeoutMs: readNumber(
			argv,
			"--sleep-log-timeout-ms",
			"SQLITE_MEMORY_SOAK_SLEEP_LOG_TIMEOUT_MS",
			10_000,
		),
		preWorkloadWaitMs: readNumber(
			argv,
			"--pre-workload-wait-ms",
			"SQLITE_MEMORY_SOAK_PRE_WORKLOAD_WAIT_MS",
			0,
		),
		postChurnWaitMs: readNumber(
			argv,
			"--post-churn-wait-ms",
			"SQLITE_MEMORY_SOAK_POST_CHURN_WAIT_MS",
			0,
		),
		postCleanupWaitMs: readNumber(
			argv,
			"--post-cleanup-wait-ms",
			"SQLITE_MEMORY_SOAK_POST_CLEANUP_WAIT_MS",
			5000,
		),
		requestLifespanSeconds: 0,
		serverlessMaxStartPayloadBytes: readNumber(
			argv,
			"--serverless-max-start-payload-bytes",
			"SQLITE_MEMORY_SOAK_SERVERLESS_MAX_START_PAYLOAD_BYTES",
			16 * 1024 * 1024,
		),
		outputDir:
			readFlag(argv, "--output-dir") ??
			process.env.SQLITE_MEMORY_SOAK_OUTPUT_DIR ??
			DEFAULT_OUTPUT_DIR,
		metricsToken:
			readFlag(argv, "--metrics-token") ??
			process.env.SQLITE_MEMORY_SOAK_METRICS_TOKEN ??
			"dev-metrics",
		reset: !argv.includes("--no-reset"),
		cleanup: !argv.includes("--no-cleanup"),
		forceGcSamples: argv.includes("--force-gc-samples"),
		keepStorage: argv.includes("--keep-storage"),
	};
	if (
		args.durationMs > 0 &&
		readFlag(argv, "--cycles") === undefined &&
		process.env.SQLITE_MEMORY_SOAK_CYCLES === undefined
	) {
		args.cycles = Number.MAX_SAFE_INTEGER;
	}
	args.requestLifespanSeconds = readNumber(
		argv,
		"--request-lifespan-seconds",
		"SQLITE_MEMORY_SOAK_REQUEST_LIFESPAN_SECONDS",
		defaultRequestLifespanSeconds(args),
	);

	for (const [name, value] of [
		["--actors", args.actors],
		["--cycles", args.cycles],
		["--cycle-interval-ms", args.cycleIntervalMs],
			["--actor-start-interval-ms", args.actorStartIntervalMs],
			["--concurrency", args.concurrency],
			[
				"--spike-max-concurrency",
				args.spikeMaxConcurrency > 0
					? args.spikeMaxConcurrency
					: args.spikeMinConcurrency > 0
						? args.concurrency
						: 1,
			],
			["--spike-period-ms", args.spikePeriodMs],
			["--insert-rows", args.insertRows],
			["--row-bytes", args.rowBytes],
			["--scan-rows", args.scanRows],
		["--sample-interval-ms", args.sampleIntervalMs],
		["--request-lifespan-seconds", args.requestLifespanSeconds],
		["--sleep-log-timeout-ms", args.sleepLogTimeoutMs],
		[
			"--serverless-max-start-payload-bytes",
			args.serverlessMaxStartPayloadBytes,
		],
	] as const) {
		if (value < 1) throw new Error(`${name} must be >= 1`);
	}
	if (args.churnSleepAfterMs < 0) {
		throw new Error("--churn-sleep-after-ms must be >= 0");
	}
	if (args.preWorkloadWaitMs < 0) {
		throw new Error("--pre-workload-wait-ms must be >= 0");
	}
	if (args.spikeMinConcurrency > 0) {
		if (args.spikeMaxConcurrency === 0) {
			args.spikeMaxConcurrency = args.concurrency;
		}
		if (args.spikeMaxConcurrency < args.spikeMinConcurrency) {
			throw new Error(
				"--spike-max-concurrency must be >= --spike-min-concurrency",
			);
		}
	}

	return args;
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function resolveEngineBinary(): string {
	if (process.env.RIVET_ENGINE_BINARY) return process.env.RIVET_ENGINE_BINARY;
	if (existsSync(REPO_ENGINE_BINARY)) return REPO_ENGINE_BINARY;
	throw new Error(
		`No local rivet-engine binary found. Build one with cargo build -p rivet-engine or set RIVET_ENGINE_BINARY.`,
	);
}

async function findOpenPort(): Promise<number> {
	return new Promise((resolvePort, reject) => {
		const server = createServer();
		server.on("error", reject);
		server.listen(0, "127.0.0.1", () => {
			const address = server.address();
			if (address === null || typeof address === "string") {
				server.close(() => reject(new Error("failed to allocate open port")));
				return;
			}
			const port = address.port;
			server.close(() => resolvePort(port));
		});
	});
}

async function waitForHttpOk(
	url: string,
	label: string,
	child: ChildProcess,
	logs: string[],
	timeoutMs = 20_000,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	let lastError: unknown;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(`${label} exited before ready:\n${logs.join("")}`);
		}

		try {
			const response = await fetch(url);
			if (response.ok) return;
			lastError = new Error(`${label} returned ${response.status}`);
		} catch (err) {
			lastError = err;
		}

		await sleep(100);
	}

	throw lastError instanceof Error
		? lastError
		: new Error(`timed out waiting for ${label}`);
}

function attachLogs(
	child: ChildProcess,
	label: string,
	logPath: string,
	logs: string[],
) {
	const append = (chunk: Buffer) => {
		const text = chunk.toString();
		logs.push(text);
		if (logs.length > 200) logs.splice(0, logs.length - 200);
		appendFileSync(logPath, text);
	};
	child.stdout?.on("data", append);
	child.stderr?.on("data", append);
	child.once("exit", (code, signal) => {
		append(
			Buffer.from(
				JSON.stringify({
					kind: "child_exit",
					label,
					code,
					signal,
					timestamp: new Date().toISOString(),
				}) + "\n",
			),
		);
	});
}

async function startEngine(args: Args, runDir: string): Promise<LocalEngine> {
	const endpointUrl = new URL(args.endpoint);
	const guardHost = endpointUrl.hostname || "127.0.0.1";
	const guardPort = Number.parseInt(endpointUrl.port, 10);
	if (!Number.isFinite(guardPort) || guardPort <= 0) {
		throw new Error(`endpoint must include a numeric port: ${args.endpoint}`);
	}

	const dbRoot = mkdtempSync(join(tmpdir(), "sqlite-memory-soak-engine-"));
	const configPath = join(runDir, "engine.config.json");
	const logPath = join(runDir, "engine.log");
	const logs: string[] = [];
	writeFileSync(
		configPath,
		`${JSON.stringify(
			{
				topology: {
					datacenter_label: 1,
					datacenters: {
						default: {
							datacenter_label: 1,
							is_leader: true,
							public_url: `${args.endpoint.replace(/\/$/, "")}/`,
							peer_url: `http://${guardHost}:${guardPort + 1}/`,
							proxy_url: null,
							valid_hosts: null,
						},
					},
				},
			},
			null,
			2,
		)}\n`,
	);
	const env: NodeJS.ProcessEnv = {
		...process.env,
		RIVET__GUARD__HOST: guardHost,
		RIVET__GUARD__PORT: guardPort.toString(),
		RIVET__API_PEER__HOST: guardHost,
		RIVET__API_PEER__PORT: (guardPort + 1).toString(),
		RIVET__METRICS__HOST: guardHost,
		RIVET__METRICS__PORT: (guardPort + 10).toString(),
		RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
		_RIVET_METRICS_TOKEN: args.metricsToken,
		MALLOC_ARENA_MAX: process.env.MALLOC_ARENA_MAX ?? "2",
		MALLOC_TRIM_THRESHOLD_: process.env.MALLOC_TRIM_THRESHOLD_ ?? "131072",
	};
	const child = spawn(resolveEngineBinary(), ["start", "--config", configPath], {
		env,
		stdio: ["ignore", "pipe", "pipe"],
	});
	attachLogs(child, "engine", logPath, logs);

	try {
		await waitForHttpOk(
			`${args.endpoint.replace(/\/$/, "")}/health`,
			"rivet-engine",
			child,
			logs,
		);
		return { child, label: "engine", logPath, logs, dbRoot };
	} catch (err) {
		await stopChild({ child, label: "engine", logPath, logs });
		rmSync(dbRoot, { recursive: true, force: true });
		throw err;
	}
}

async function startKitchenSinkServer(
	args: Args,
	runDir: string,
	serverPort: number,
): Promise<ManagedChild> {
	const logPath = join(runDir, "kitchen-sink.log");
	const logs: string[] = [];
	const serverlessUrl = `http://127.0.0.1:${serverPort}/api/rivet`;
	const env: NodeJS.ProcessEnv = {
		...process.env,
		PORT: serverPort.toString(),
		RIVET_ENDPOINT: args.endpoint,
		RIVET_TOKEN: process.env.RIVET_TOKEN ?? "dev",
		RIVET_NAMESPACE: process.env.RIVET_NAMESPACE ?? "default",
		RIVET_POOL: process.env.RIVET_POOL ?? "default",
		RIVET_SERVERLESS_URL: serverlessUrl,
		RIVET_SERVERLESS_REQUEST_LIFESPAN:
			args.requestLifespanSeconds.toString(),
		RIVET_SERVERLESS_DRAIN_GRACE_PERIOD:
			process.env.RIVET_SERVERLESS_DRAIN_GRACE_PERIOD ?? "5",
		RIVET_SERVERLESS_MAX_START_PAYLOAD_BYTES:
			args.serverlessMaxStartPayloadBytes.toString(),
		SQLITE_MEMORY_SOAK_DIAGNOSTICS: "1",
		MALLOC_ARENA_MAX: process.env.MALLOC_ARENA_MAX ?? "2",
		MALLOC_TRIM_THRESHOLD_: process.env.MALLOC_TRIM_THRESHOLD_ ?? "131072",
	};
	delete env.RIVET_RUN_ENGINE;

	const nodeArgs = [
		...(args.forceGcSamples ? ["--expose-gc"] : []),
		"--import",
		"@rivetkit/sql-loader",
		"--import",
		"tsx",
		"src/server.ts",
	];
	const command =
		process.env.SQLITE_MEMORY_SOAK_STRACE === "1" ? "strace" : process.execPath;
	const commandArgs =
		process.env.SQLITE_MEMORY_SOAK_STRACE === "1"
			? [
					"-ff",
					"-tt",
					"-e",
					"trace=none",
					"-e",
					"signal=all",
					"-o",
					join(runDir, "kitchen-sink.strace"),
					process.execPath,
					...nodeArgs,
				]
			: nodeArgs;
	const child = spawn(command, commandArgs, {
		cwd: EXAMPLE_DIR,
		env,
		stdio: ["ignore", "pipe", "pipe"],
	});
	attachLogs(child, "kitchen-sink", logPath, logs);

	try {
		await waitForHttpOk(
			`http://127.0.0.1:${serverPort}/debug/memory`,
			"kitchen-sink",
			child,
			logs,
		);
		await waitForHttpOk(
			`${serverlessUrl}/metadata`,
			"kitchen-sink metadata",
			child,
			logs,
		);
		await configureServerlessRunner(args, serverlessUrl);
		return { child, label: "kitchen-sink", logPath, logs };
	} catch (err) {
		await stopChild({ child, label: "kitchen-sink", logPath, logs });
		throw err;
	}
}

async function configureServerlessRunner(
	args: Args,
	serverlessUrl: string,
): Promise<void> {
	const base = args.endpoint.replace(/\/$/, "");
	const namespace = process.env.RIVET_NAMESPACE ?? "default";
	const token = process.env.RIVET_TOKEN ?? "dev";
	const poolName = process.env.RIVET_POOL ?? "default";
	const datacentersResponse = await fetch(`${base}/datacenters?namespace=${namespace}`, {
		headers: { Authorization: `Bearer ${token}` },
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

	const response = await fetch(
		`${base}/runner-configs/${poolName}?namespace=${namespace}`,
		{
			method: "PUT",
			headers: {
				Authorization: `Bearer ${token}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				datacenters: {
					[datacenter]: {
						serverless: {
							url: serverlessUrl,
							headers: {
								"x-rivet-token": token,
							},
							request_lifespan: args.requestLifespanSeconds,
							drain_grace_period: 5,
							metadata_poll_interval: 1000,
							max_runners: 100_000,
							min_runners: 0,
							runners_margin: 0,
							slots_per_runner: 1,
						},
						metadata: {
							source: "kitchen-sink",
							workload: "sqlite-memory-soak",
						},
						drain_on_version_upgrade: true,
					},
				},
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`failed to configure local serverless runner: ${response.status} ${await response.text()}`,
		);
	}
}

async function stopChild(managed: ManagedChild | undefined): Promise<void> {
	if (!managed) return;
	const { child, label, logPath, logs } = managed;
	if (child.exitCode !== null) return;

	const event =
		JSON.stringify({
			kind: "harness_stop_child",
			label,
			pid: child.pid,
			timestamp: new Date().toISOString(),
			stack: new Error().stack,
		}) + "\n";
	logs.push(event);
	appendFileSync(logPath, event);
	child.kill("SIGTERM");
	await Promise.race([
		new Promise<void>((resolveExit) => child.once("exit", () => resolveExit())),
		sleep(5_000),
	]);
	if (child.exitCode === null) child.kill("SIGKILL");
}

async function stopEngine(
	engine: LocalEngine | undefined,
	keepStorage: boolean,
): Promise<void> {
	if (!engine) return;
	await stopChild(engine);
	if (!keepStorage) rmSync(engine.dbRoot, { recursive: true, force: true });
}

function parseKbLine(text: string, name: string): number | null {
	const match = new RegExp(`^${name}:\\s+(\\d+)\\s+kB$`, "m").exec(text);
	if (!match) return null;
	return Number.parseInt(match[1]!, 10) * 1024;
}

function parseNumberLine(text: string, name: string): number | null {
	const match = new RegExp(`^${name}:\\s+(\\d+)$`, "m").exec(text);
	if (!match) return null;
	return Number.parseInt(match[1]!, 10);
}

function readSmapsRollup(pid: number): Record<string, number> | undefined {
	try {
		const text = readFileSync(`/proc/${pid}/smaps_rollup`, "utf8");
		const result: Record<string, number> = {};
		for (const line of text.split("\n")) {
			const match = /^([A-Za-z_]+):\s+(\d+)\s+kB$/.exec(line);
			if (match) result[match[1]!] = Number.parseInt(match[2]!, 10) * 1024;
		}
		return result;
	} catch {
		return undefined;
	}
}

function parseProcStat(pid: number): {
	procState: string | null;
	cpuUserSeconds: number | null;
	cpuSystemSeconds: number | null;
	cpuTotalSeconds: number | null;
	threads: number | null;
} {
	try {
		const text = readFileSync(`/proc/${pid}/stat`, "utf8").trim();
		const closeParen = text.lastIndexOf(")");
		if (closeParen === -1) throw new Error("missing comm terminator");
		const fields = text.slice(closeParen + 2).split(" ");
		const utime = Number.parseInt(fields[11] ?? "", 10);
		const stime = Number.parseInt(fields[12] ?? "", 10);
		const threads = Number.parseInt(fields[17] ?? "", 10);
		const cpuUserSeconds = Number.isFinite(utime)
			? utime / CLOCK_TICKS_PER_SECOND
			: null;
		const cpuSystemSeconds = Number.isFinite(stime)
			? stime / CLOCK_TICKS_PER_SECOND
			: null;
		return {
			procState: fields[0] ?? null,
			cpuUserSeconds,
			cpuSystemSeconds,
			cpuTotalSeconds:
				cpuUserSeconds !== null && cpuSystemSeconds !== null
					? cpuUserSeconds + cpuSystemSeconds
					: null,
			threads: Number.isFinite(threads) ? threads : null,
		};
	} catch {
		return {
			procState: null,
			cpuUserSeconds: null,
			cpuSystemSeconds: null,
			cpuTotalSeconds: null,
			threads: null,
		};
	}
}

function parseProcIo(pid: number): ProcIo | undefined {
	try {
		const text = readFileSync(`/proc/${pid}/io`, "utf8");
		const field = (name: string) => {
			const match = new RegExp(`^${name}:\\s+(\\d+)$`, "m").exec(text);
			return match ? Number.parseInt(match[1]!, 10) : null;
		};
		return {
			readBytes: field("read_bytes"),
			writeBytes: field("write_bytes"),
			syscr: field("syscr"),
			syscw: field("syscw"),
		};
	} catch {
		return undefined;
	}
}

function countOpenFds(pid: number): number | null {
	try {
		return readdirSync(`/proc/${pid}/fd`).length;
	} catch {
		return null;
	}
}

function readProcMemory(pid: number | undefined): ProcMemory {
	if (pid === undefined) {
		return {
			pid: null,
			alive: false,
			rssBytes: null,
			hwmRssBytes: null,
			vmSizeBytes: null,
			threads: null,
			error: "missing pid",
		};
	}

	try {
		const status = readFileSync(`/proc/${pid}/status`, "utf8");
		const stat = parseProcStat(pid);
		return {
			pid,
			alive: true,
			rssBytes: parseKbLine(status, "VmRSS"),
			hwmRssBytes: parseKbLine(status, "VmHWM"),
			vmSizeBytes: parseKbLine(status, "VmSize"),
			threads: stat.threads ?? parseNumberLine(status, "Threads"),
			procState: stat.procState,
			cpuUserSeconds: stat.cpuUserSeconds,
			cpuSystemSeconds: stat.cpuSystemSeconds,
			cpuTotalSeconds: stat.cpuTotalSeconds,
			openFds: countOpenFds(pid),
			io: parseProcIo(pid),
			smapsRollup: readSmapsRollup(pid),
		};
	} catch (err) {
		return {
			pid,
			alive: false,
			rssBytes: null,
			hwmRssBytes: null,
			vmSizeBytes: null,
			threads: null,
			error: err instanceof Error ? err.message : String(err),
		};
	}
}

async function fetchKitchenSinkBreakdown(
	serverPort: number,
	forceGc: boolean,
): Promise<unknown> {
	try {
		const url = new URL(`http://127.0.0.1:${serverPort}/debug/memory`);
		if (forceGc) url.searchParams.set("gc", "1");
		const response = await fetch(url);
		if (!response.ok) {
			return { error: `status ${response.status}`, body: await response.text() };
		}
		return await response.json();
	} catch (err) {
		return { error: err instanceof Error ? err.message : String(err) };
	}
}

async function captureKitchenSinkHeapSnapshot(
	serverPort: number,
	jsonlPath: string,
	path: string,
	label: string,
) {
	const url = new URL(`http://127.0.0.1:${serverPort}/debug/heap-snapshot`);
	url.searchParams.set("path", path);
	const response = await fetch(url, { method: "POST" });
	const body = await response.text();
	writeEvent(jsonlPath, {
		kind: "heap_snapshot",
		label,
		path,
		status: response.status,
		body,
		timestamp: new Date().toISOString(),
	});
	if (!response.ok) {
		throw new Error(`failed to capture heap snapshot ${label}: ${response.status} ${body}`);
	}
}

function writeEvent(jsonlPath: string, event: unknown) {
	appendFileSync(jsonlPath, `${JSON.stringify(event)}\n`);
}

function logOffset(logPath: string): number {
	try {
		return statSync(logPath).size;
	} catch {
		return 0;
	}
}

function readLogSince(logPath: string, offset: number): string {
	const text = readFileSync(logPath, "utf8");
	return text.slice(Math.min(offset, text.length));
}

async function waitForActorSleepLog(
	server: ManagedChild,
	actorId: string,
	offset: number,
	timeoutMs: number,
): Promise<{ matched: string }> {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		if (server.child.exitCode !== null) {
			throw new Error(
				`kitchen-sink exited before actor sleep log was observed for ${actorId}`,
			);
		}

		const text = readLogSince(server.logPath, offset);
		const matched = text
			.split("\n")
			.find(
				(line) =>
					line.includes(actorId) &&
					(line.includes("sqlite_memory_pressure_on_sleep") ||
						line.includes("actor sleeping")),
			);
		if (matched) return { matched };

		await sleep(100);
	}

	throw new Error(`timed out waiting for actor sleeping log for ${actorId}`);
}

async function forceActorSleepViaApi(
	args: Args,
	actorId: string,
): Promise<unknown> {
	const namespace = process.env.RIVET_NAMESPACE ?? "default";
	const token = process.env.RIVET_TOKEN ?? "dev";
	const response = await fetch(
		`${args.endpoint.replace(/\/$/, "")}/actors/${encodeURIComponent(actorId)}/sleep?namespace=${encodeURIComponent(namespace)}`,
		{
			method: "POST",
			headers: {
				Authorization: `Bearer ${token}`,
				"Content-Type": "application/json",
			},
			body: "{}",
		},
	);
	const bodyText = await response.text();
	if (!response.ok) {
		throw new Error(
			`failed to force actor sleep: ${response.status} ${bodyText}`,
		);
	}
	if (!bodyText) return null;
	try {
		return JSON.parse(bodyText) as unknown;
	} catch {
		return bodyText;
	}
}

async function captureSample(
	args: Args,
	runId: string,
	startedAt: number,
	jsonlPath: string,
	engine: LocalEngine,
	server: ManagedChild,
	serverPort: number,
	samples: MemorySample[],
): Promise<void> {
	const kitchenSinkBreakdown = await fetchKitchenSinkBreakdown(
		serverPort,
		args.forceGcSamples,
	);
	const sample: MemorySample = {
		kind: "memory_sample",
		runId,
		elapsedMs: Date.now() - startedAt,
		timestamp: new Date().toISOString(),
		harness: readProcMemory(process.pid),
		engine: readProcMemory(engine.child.pid),
		kitchenSink: readProcMemory(
			kitchenSinkPidFromBreakdown(kitchenSinkBreakdown) ?? server.child.pid,
		),
		kitchenSinkBreakdown,
	};
	samples.push(sample);
	writeEvent(jsonlPath, sample);
}

async function sampleLoop(
	args: Args,
	runId: string,
	startedAt: number,
	jsonlPath: string,
	engine: LocalEngine,
	server: ManagedChild,
	serverPort: number,
	samples: MemorySample[],
	shouldStop: () => boolean,
): Promise<void> {
	while (!shouldStop()) {
		await captureSample(
			args,
			runId,
			startedAt,
			jsonlPath,
			engine,
			server,
			serverPort,
			samples,
		);
		await sleep(args.sampleIntervalMs);
	}
}

function assertCycle(result: {
	integrityCheck: string;
	activeRows: number;
	activeBytes: number;
}) {
	if (result.integrityCheck !== "ok") {
		throw new Error(`sqlite integrity check failed: ${result.integrityCheck}`);
	}
	if (result.activeRows < 0 || result.activeBytes < 0) {
		throw new Error(`invalid actor stats: ${JSON.stringify(result)}`);
	}
}

async function runActorDriver(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	actorIndex: number,
	workloadStartedAt: number,
	jsonlPath: string,
): Promise<void> {
	const key = ["sqlite-memory-soak", args.seed, String(actorIndex)];
	const handle = client.sqliteMemoryPressure.getOrCreate(key);
	const actorOffsetMs = Math.floor(
		(args.cycleIntervalMs * actorIndex) / args.actors,
	);
	const stopAt =
		args.durationMs > 0 ? workloadStartedAt + args.durationMs : Number.POSITIVE_INFINITY;
	let wroteActorWake = false;

	for (let cycle = 0; cycle < args.cycles; cycle += 1) {
		const scheduledAt =
			workloadStartedAt + actorOffsetMs + cycle * args.cycleIntervalMs;
		if (scheduledAt >= stopAt) break;

		const waitMs = scheduledAt - Date.now();
		if (waitMs > 0) {
			await sleep(waitMs);
		}
		if (Date.now() >= stopAt) {
			writeEvent(jsonlPath, {
				kind: "cycle_skipped_after_duration",
				actorIndex,
				key,
				cycle,
				scheduledAt: new Date(scheduledAt).toISOString(),
				timestamp: new Date().toISOString(),
			});
			break;
		}

		const lateMs = Math.max(0, Date.now() - scheduledAt);
		const startedAt = performance.now();
		const result = await handle.runCycle({
			seed: `${args.seed}:${actorIndex}`,
			cycle,
			insertRows: args.insertRows,
			rowBytes: args.rowBytes,
			scanRows: args.scanRows,
		});
		assertCycle(result);
		const durationMs = performance.now() - startedAt;
		if (!wroteActorWake) {
			wroteActorWake = true;
			writeEvent(jsonlPath, {
				kind: "actor_wake",
				actorIndex,
				key,
				source: "first_cycle",
				timestamp: new Date().toISOString(),
			});
		}
		writeEvent(jsonlPath, {
			kind: "cycle",
			actorIndex,
			key,
			cycle,
			scheduledAt: new Date(scheduledAt).toISOString(),
			lateMs,
			durationMs,
			result,
			timestamp: new Date().toISOString(),
		});
		console.log(
			`cycle ok actor=${actorIndex} cycle=${cycle} rows=${result.activeRows} bytes=${result.activeBytes} pages=${result.storage.page_count} ms=${durationMs.toFixed(1)}`,
		);

		if (args.wakeEvery > 0 && (cycle + 1) % args.wakeEvery === 0) {
			await handle.goToSleep();
			await sleep(args.wakeDelayMs);
			const stats = await handle.stats();
			if (stats.integrityCheck !== "ok") {
				throw new Error(
					`sqlite integrity check failed after wake: ${stats.integrityCheck}`,
				);
			}
			writeEvent(jsonlPath, {
				kind: "wake",
				actorIndex,
				key,
				cycle,
				stats,
				timestamp: new Date().toISOString(),
			});
		}
	}
}

async function runChurnActorDriver(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	server: ManagedChild,
	actorIndex: number,
	jsonlPath: string,
): Promise<void> {
	const key = ["sqlite-memory-soak", args.seed, String(actorIndex)];
	const handle = client.sqliteMemoryPressure.getOrCreate(key);
	if (args.reset) {
		const resetStartedAt = performance.now();
		const reset = await handle.reset();
		writeEvent(jsonlPath, {
			kind: "actor_wake",
			actorIndex,
			key,
			source: "reset",
			timestamp: new Date().toISOString(),
		});
		writeEvent(jsonlPath, {
			kind: "actor_reset",
			actorIndex,
			key,
			durationMs: performance.now() - resetStartedAt,
			reset,
			timestamp: new Date().toISOString(),
		});
	}

	const actorStartedAt = Date.now();
	const sleepAt = actorStartedAt + args.churnSleepAfterMs;
	const stopAt =
		args.durationMs > 0 ? actorStartedAt + args.durationMs : Number.POSITIVE_INFINITY;

	for (let cycle = 0; cycle < args.cycles; cycle += 1) {
		const scheduledAt = actorStartedAt + cycle * args.cycleIntervalMs;
		if (scheduledAt >= sleepAt || scheduledAt >= stopAt) break;

		const waitMs = scheduledAt - Date.now();
		if (waitMs > 0) await sleep(waitMs);
		if (Date.now() >= sleepAt || Date.now() >= stopAt) break;

		const lateMs = Math.max(0, Date.now() - scheduledAt);
		const startedAt = performance.now();
		const result = await handle.runCycle({
			seed: `${args.seed}:${actorIndex}`,
			cycle,
			insertRows: args.insertRows,
			rowBytes: args.rowBytes,
			scanRows: args.scanRows,
		});
		assertCycle(result);
		const durationMs = performance.now() - startedAt;
		writeEvent(jsonlPath, {
			kind: "cycle",
			actorIndex,
			key,
			cycle,
			scheduledAt: new Date(scheduledAt).toISOString(),
			lateMs,
			durationMs,
			result,
			timestamp: new Date().toISOString(),
		});
		console.log(
			`cycle ok actor=${actorIndex} cycle=${cycle} rows=${result.activeRows} bytes=${result.activeBytes} pages=${result.storage.page_count} ms=${durationMs.toFixed(1)}`,
		);
	}

	const remainingMs = sleepAt - Date.now();
	if (remainingMs > 0) await sleep(remainingMs);

	const actorId = await handle.resolve();
	const logStart = logOffset(server.logPath);
	const sleepStartedAt = performance.now();
	const response = await forceActorSleepViaApi(args, actorId);
	writeEvent(jsonlPath, {
		kind: "actor_api_sleep",
		actorIndex,
		key,
		actorId,
		durationMs: performance.now() - sleepStartedAt,
		response,
		timestamp: new Date().toISOString(),
	});
	const verified = await waitForActorSleepLog(
		server,
		actorId,
		logStart,
		args.sleepLogTimeoutMs,
	);
	writeEvent(jsonlPath, {
		kind: "actor_sleep_verified",
		actorIndex,
		key,
		actorId,
		log: verified.matched,
		timestamp: new Date().toISOString(),
	});
	console.log(`actor sleep verified actor=${actorIndex} actor_id=${actorId}`);
}

async function resetActorOnSchedule(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	actorIndex: number,
	startupStartedAt: number,
	jsonlPath: string,
): Promise<void> {
	const scheduledAt = startupStartedAt + actorIndex * args.actorStartIntervalMs;
	const waitMs = scheduledAt - Date.now();
	if (waitMs > 0) {
		await sleep(waitMs);
	}

	const key = ["sqlite-memory-soak", args.seed, String(actorIndex)];
	const handle = client.sqliteMemoryPressure.getOrCreate(key);
	const resetStartedAt = performance.now();
	const reset = await handle.reset();
	writeEvent(jsonlPath, {
		kind: "actor_reset",
		actorIndex,
		key,
		scheduledAt: new Date(scheduledAt).toISOString(),
		lateMs: Math.max(0, Date.now() - scheduledAt),
		durationMs: performance.now() - resetStartedAt,
		reset,
		timestamp: new Date().toISOString(),
	});
}

async function resetActors(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	jsonlPath: string,
): Promise<void> {
	if (!args.reset) return;

	let nextActor = 0;
	const startupStartedAt = Date.now();

	async function worker(workerId: number) {
		for (;;) {
			const actorIndex = nextActor;
			nextActor += 1;
			if (actorIndex >= args.actors) return;

			console.log(`actor reset start worker=${workerId} actor=${actorIndex}`);
			await resetActorOnSchedule(
				args,
				client,
				actorIndex,
				startupStartedAt,
				jsonlPath,
			);
		}
	}

	await Promise.all(
		Array.from(
			{ length: Math.min(args.concurrency, args.actors) },
			(_, workerId) => worker(workerId),
		),
	);
}

function spikeModeEnabled(args: Args): boolean {
	return args.spikeMinConcurrency > 0;
}

function targetConcurrencyForElapsed(args: Args, elapsedMs: number): number {
	if (!spikeModeEnabled(args)) return args.concurrency;

	const min = args.spikeMinConcurrency;
	const max = args.spikeMaxConcurrency;
	if (min === max) return min;

	const phase = (elapsedMs % args.spikePeriodMs) / args.spikePeriodMs;
	const wave = phase < 0.5 ? phase * 2 : (1 - phase) * 2;
	return Math.round(min + (max - min) * wave);
}

async function runWithSpikeConcurrency(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	server: ManagedChild,
	jsonlPath: string,
): Promise<void> {
	if (args.durationMs <= 0) {
		throw new Error("--duration-ms is required with spike concurrency");
	}

	let nextActor = 0;
	let active = 0;
	let completed = 0;
	let failed = false;
	const errors: unknown[] = [];
	const startedAt = Date.now();
	const stopSchedulingAt = startedAt + args.durationMs;
	const workers = new Set<Promise<void>>();

	function spawnActor(actorIndex: number) {
		active += 1;
		let worker: Promise<void>;
		worker = runChurnActorDriver(
			args,
			client,
			server,
			actorIndex,
			jsonlPath,
		)
			.catch((err) => {
				failed = true;
				errors.push(err);
			})
			.finally(() => {
				active -= 1;
				completed += 1;
				workers.delete(worker);
			});
		workers.add(worker);
	}

	while (Date.now() < stopSchedulingAt && nextActor < args.actors && !failed) {
		const elapsedMs = Date.now() - startedAt;
		const target = targetConcurrencyForElapsed(args, elapsedMs);
		writeEvent(jsonlPath, {
			kind: "concurrency_target",
			elapsedMs,
			target,
			active,
			completed,
			nextActor,
			timestamp: new Date().toISOString(),
		});

		while (active < target && nextActor < args.actors) {
			const actorIndex = nextActor;
			nextActor += 1;
			console.log(
				`actor spike start actor=${actorIndex} active=${active + 1} target=${target}`,
			);
			spawnActor(actorIndex);
		}

		await sleep(Math.min(250, Math.max(25, args.cycleIntervalMs)));
	}

	await Promise.allSettled(workers);
	if (errors.length > 0) {
		const first = errors[0];
		throw first instanceof Error ? first : new Error(String(first));
	}
	if (nextActor >= args.actors) {
		throw new Error(
			`ran out of actors before spike duration completed: actors=${args.actors}`,
		);
	}
}

async function runWithActorConcurrency(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	server: ManagedChild,
	jsonlPath: string,
): Promise<void> {
	let nextActor = 0;
	if (args.churnSleepAfterMs > 0) {
		if (spikeModeEnabled(args)) {
			await runWithSpikeConcurrency(args, client, server, jsonlPath);
			return;
		}

		async function churnWorker(workerId: number) {
			for (;;) {
				const actorIndex = nextActor;
				nextActor += 1;
				if (actorIndex >= args.actors) return;

				console.log(`actor churn start worker=${workerId} actor=${actorIndex}`);
				await runChurnActorDriver(
					args,
					client,
					server,
					actorIndex,
					jsonlPath,
				);
			}
		}

		await Promise.all(
			Array.from(
				{ length: Math.min(args.concurrency, args.actors) },
				(_, workerId) => churnWorker(workerId),
			),
		);
		return;
	}

	await resetActors(args, client, jsonlPath);

	const workloadStartedAt = Date.now() + args.cycleIntervalMs;

	async function worker(workerId: number) {
		for (;;) {
			const actorIndex = nextActor;
			nextActor += 1;
			if (actorIndex >= args.actors) return;

			console.log(`actor driver start worker=${workerId} actor=${actorIndex}`);
			await runActorDriver(
				args,
				client,
				actorIndex,
				workloadStartedAt,
				jsonlPath,
			);
		}
	}

	await Promise.all(
		Array.from(
			{ length: Math.min(args.concurrency, args.actors) },
			(_, workerId) => worker(workerId),
		),
	);
}

async function cleanupActors(
	args: Args,
	client: ReturnType<typeof createClient<typeof registry>>,
	jsonlPath: string,
): Promise<void> {
	if (!args.cleanup) return;

	await Promise.all(
		Array.from({ length: args.actors }, async (_, actorIndex) => {
			const key = ["sqlite-memory-soak", args.seed, String(actorIndex)];
			const handle = client.sqliteMemoryPressure.getOrCreate(key);
			const reset = await handle.reset();
			const sleep = await handle.goToSleep();
			writeEvent(jsonlPath, {
				kind: "actor_cleanup",
				actorIndex,
				key,
				reset,
				sleep,
				timestamp: new Date().toISOString(),
			});
		}),
	);
}

function bytesToMiB(bytes: number | null): string {
	if (bytes === null) return "n/a";
	return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

function summarizeProcess(
	label: string,
	samples: MemorySample[],
	select: (sample: MemorySample) => ProcMemory,
): string {
	const rssValues = samples
		.map((sample) => select(sample).rssBytes)
		.filter((value): value is number => typeof value === "number");
	if (rssValues.length === 0) return `${label}: no samples`;

	const first = rssValues[0]!;
	const final = rssValues[rssValues.length - 1]!;
	const max = Math.max(...rssValues);
	return `${label}: start=${bytesToMiB(first)} max=${bytesToMiB(max)} final=${bytesToMiB(final)} delta=${bytesToMiB(final - first)}`;
}

function summarizeKitchenBreakdown(samples: MemorySample[]): string | undefined {
	const breakdowns = samples
		.map((sample) => sample.kitchenSinkBreakdown)
		.filter((value): value is { estimates?: Record<string, number> } => {
			return typeof value === "object" && value !== null && "estimates" in value;
		});
	if (breakdowns.length === 0) return undefined;

	const first = breakdowns[0]!.estimates ?? {};
	const final = breakdowns[breakdowns.length - 1]!.estimates ?? {};
	return [
		"kitchen estimates:",
		`jsHeapUsed ${bytesToMiB(first.jsHeapUsedBytes ?? null)} -> ${bytesToMiB(final.jsHeapUsedBytes ?? null)}`,
		`v8External ${bytesToMiB(first.v8ExternalBytes ?? null)} -> ${bytesToMiB(final.v8ExternalBytes ?? null)}`,
		`nativeNonV8 ${bytesToMiB(first.nativeNonV8ResidentEstimateBytes ?? null)} -> ${bytesToMiB(final.nativeNonV8ResidentEstimateBytes ?? null)}`,
	].join(" ");
}

function summarizeCycleVfs(jsonlPath: string): string | undefined {
	let count = 0;
	let final:
		| {
				actorIndex: number;
				cycle: number;
				result?: {
					storage?: {
						page_count?: number;
						page_size?: number;
						vfs?: {
							pageCacheEntries?: number;
							pageCacheWeightedSize?: number;
							pageCacheCapacityPages?: number;
							writeBufferDirtyPages?: number;
							dbSizePages?: number;
						};
					};
				};
		  }
		| undefined;
	let totalCacheEntries = 0;
	let totalCacheBytes = 0;
	let totalDbPages = 0;
	for (const line of readFileSync(jsonlPath, "utf8").split("\n")) {
		if (!line) continue;
		const event = JSON.parse(line) as {
			kind?: string;
			actorIndex?: number;
			cycle?: number;
			result?: {
				storage?: {
					page_count?: number;
					page_size?: number;
					vfs?: {
						pageCacheEntries?: number;
						pageCacheWeightedSize?: number;
						pageCacheCapacityPages?: number;
						writeBufferDirtyPages?: number;
						dbSizePages?: number;
					};
				};
			};
		};
		if (event.kind !== "cycle") continue;
		const vfs = event.result?.storage?.vfs;
		if (!vfs || typeof vfs.pageCacheEntries !== "number") continue;
		count++;
		final = event as typeof final;
		totalCacheEntries += vfs.pageCacheEntries;
		totalCacheBytes +=
			(vfs.pageCacheWeightedSize ?? 0) *
			(event.result?.storage?.page_size ?? SQLITE_PAGE_SIZE_BYTES);
		totalDbPages += vfs.dbSizePages ?? event.result?.storage?.page_count ?? 0;
	}
	if (!final || count === 0) return undefined;
	const avgCacheBytes = totalCacheBytes / count;
	const avgCacheEntries = totalCacheEntries / count;
	const avgDbBytes = (totalDbPages / count) * 4096;
	const finalVfs = final.result?.storage?.vfs;
	return [
		"vfs:",
		`samples=${count}`,
		`avg_cache=${bytesToMiB(avgCacheBytes)}`,
		`avg_entries=${avgCacheEntries.toFixed(0)}`,
		`avg_db=${bytesToMiB(avgDbBytes)}`,
		`final_actor=${final.actorIndex}`,
		`final_cycle=${final.cycle}`,
		`final_cache=${bytesToMiB(
			finalVfs?.pageCacheWeightedSize === undefined
				? null
				: finalVfs.pageCacheWeightedSize *
						(final.result?.storage?.page_size ?? SQLITE_PAGE_SIZE_BYTES),
		)}`,
		`final_entries=${finalVfs?.pageCacheEntries ?? "n/a"}`,
	].join(" ");
}

function summarizeActorSleeps(jsonlPath: string): string | undefined {
	let requested = 0;
	let verified = 0;
	for (const line of readFileSync(jsonlPath, "utf8").split("\n")) {
		if (!line) continue;
		const event = JSON.parse(line) as { kind?: string };
		if (event.kind === "actor_api_sleep") requested++;
		if (event.kind === "actor_sleep_verified") verified++;
	}
	if (requested === 0 && verified === 0) return undefined;
	return `actor sleeps: api_requested=${requested} log_verified=${verified}`;
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	const serverPort = args.serverPort > 0 ? args.serverPort : await findOpenPort();
	const runId = args.seed.replace(/[^a-zA-Z0-9_.-]/g, "_");
	const outputRoot = resolve(REPO_ROOT, args.outputDir);
	const runDir = join(outputRoot, runId);
	const jsonlPath = join(runDir, "events.jsonl");
	mkdirSync(runDir, { recursive: true });

	let engine: LocalEngine | undefined;
	let server: ManagedChild | undefined;
	let stopSampling = false;
	const samples: MemorySample[] = [];
	const startedAt = Date.now();

	writeEvent(jsonlPath, {
		kind: "run_start",
		runId,
		args: { ...args, serverPort },
		timestamp: new Date().toISOString(),
	});

	try {
		console.log("SQLite memory soak");
		console.log(`run_id=${runId}`);
		console.log(`endpoint=${args.endpoint}`);
		console.log(`server_port=${serverPort}`);
		console.log(
			`request_lifespan_seconds=${args.requestLifespanSeconds}`,
		);
		console.log(`output=${jsonlPath}`);

		engine = await startEngine(args, runDir);
		console.log(`engine pid=${engine.child.pid} log=${engine.logPath}`);

		server = await startKitchenSinkServer(args, runDir, serverPort);
		console.log(`kitchen_sink pid=${server.child.pid} log=${server.logPath}`);

		await captureSample(
			args,
			runId,
			startedAt,
			jsonlPath,
			engine,
			server,
			serverPort,
			samples,
		);
		if (process.env.SQLITE_MEMORY_SOAK_HEAP_SNAPSHOTS === "1") {
			await captureKitchenSinkHeapSnapshot(
				serverPort,
				jsonlPath,
				join(runDir, "kitchen-sink-start.heapsnapshot"),
				"start",
			);
		}
		const sampler = sampleLoop(
			args,
			runId,
			startedAt,
			jsonlPath,
			engine,
			server,
			serverPort,
			samples,
			() => stopSampling,
		);

		const client = createClient<typeof registry>({
			endpoint: args.endpoint,
			namespace: process.env.RIVET_NAMESPACE ?? "default",
			token: process.env.RIVET_TOKEN ?? "dev",
			poolName: process.env.RIVET_POOL ?? "default",
		});

		if (args.preWorkloadWaitMs > 0) {
			writeEvent(jsonlPath, {
				kind: "pre_workload_wait_start",
				durationMs: args.preWorkloadWaitMs,
				timestamp: new Date().toISOString(),
			});
			await sleep(args.preWorkloadWaitMs);
			await captureSample(
				args,
				runId,
				startedAt,
				jsonlPath,
				engine,
				server,
				serverPort,
				samples,
			);
			writeEvent(jsonlPath, {
				kind: "pre_workload_wait_end",
				durationMs: args.preWorkloadWaitMs,
				timestamp: new Date().toISOString(),
			});
		}

		await runWithActorConcurrency(args, client, server, jsonlPath);
		await captureSample(
			args,
			runId,
			startedAt,
			jsonlPath,
			engine,
			server,
			serverPort,
			samples,
		);
		if (args.postChurnWaitMs > 0) {
			writeEvent(jsonlPath, {
				kind: "post_churn_wait_start",
				durationMs: args.postChurnWaitMs,
				timestamp: new Date().toISOString(),
			});
			await sleep(args.postChurnWaitMs);
			await captureSample(
				args,
				runId,
				startedAt,
				jsonlPath,
				engine,
				server,
				serverPort,
				samples,
			);
			writeEvent(jsonlPath, {
				kind: "post_churn_wait_end",
				durationMs: args.postChurnWaitMs,
				timestamp: new Date().toISOString(),
			});
		}
		if (process.env.SQLITE_MEMORY_SOAK_HEAP_SNAPSHOTS === "1") {
			await captureKitchenSinkHeapSnapshot(
				serverPort,
				jsonlPath,
				join(runDir, "kitchen-sink-final.heapsnapshot"),
				"final",
			);
		}
		await cleanupActors(args, client, jsonlPath);
		if (args.cleanup && args.postCleanupWaitMs > 0) {
			await sleep(args.postCleanupWaitMs);
		}
		await captureSample(
			args,
			runId,
			startedAt,
			jsonlPath,
			engine,
			server,
			serverPort,
			samples,
		);

		stopSampling = true;
		await sampler;

		writeEvent(jsonlPath, {
			kind: "run_complete",
			runId,
			durationMs: Date.now() - startedAt,
			timestamp: new Date().toISOString(),
		});

		console.log("summary");
		console.log(summarizeProcess("harness", samples, (sample) => sample.harness));
		console.log(summarizeProcess("engine", samples, (sample) => sample.engine));
		console.log(
			summarizeProcess("kitchen-sink", samples, (sample) => sample.kitchenSink),
		);
		const kitchenSummary = summarizeKitchenBreakdown(samples);
		if (kitchenSummary) console.log(kitchenSummary);
		const vfsSummary = summarizeCycleVfs(jsonlPath);
		if (vfsSummary) console.log(vfsSummary);
			const sleepSummary = summarizeActorSleeps(jsonlPath);
			if (sleepSummary) console.log(sleepSummary);
			console.log(`events=${jsonlPath}`);
	} catch (err) {
		writeEvent(jsonlPath, {
			kind: "run_error",
			runId,
			error: err instanceof Error ? err.stack ?? err.message : String(err),
			timestamp: new Date().toISOString(),
		});
		throw err;
	} finally {
		stopSampling = true;
		await stopChild(server);
		await stopEngine(engine, args.keepStorage);
	}
}

main().catch((err) => {
	console.error(err instanceof Error ? err.stack ?? err.message : err);
	process.exit(1);
});
