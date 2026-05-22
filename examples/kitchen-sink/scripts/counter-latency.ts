// Counter-latency mini load test.
//
// Subcommands:
//
// rtt              Every --interval ms, spawn a background worker that:
//                    1. Generates a new key
//                    2. handle = client.counter.getOrCreate([key])
//                    3. connection = handle.connect()
//                    4. measures connect, first increment, second increment
//                  Workers run concurrently by default. Set SERIAL=1 to force serial execution.
//
// concurrent       Ramps up to persistent raw WebSocket tunnel-stress actors.
//
// agent-concurrent Ramps up to persistent load-test agent actors. Each worker sends an
//                  inference message every --message-interval-ms. The actor inserts
//                  tokens into SQLite and streams one token event per insert.
//
// Usage:
//   tsx scripts/counter-latency.ts rtt -i <ms> <endpoint>
//   tsx scripts/counter-latency.ts concurrent [options] <endpoint>
//   tsx scripts/counter-latency.ts agent-concurrent [options] <endpoint>
//
//   BATCHES total workers spawned before exit in rtt mode. Default: infinite.
//   SERIAL  "1" / "true" to await each worker before the next in rtt mode.

import { parseArgs } from "node:util";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

interface RttArgs {
	mode: "rtt";
	interval: number;
	endpoint: string;
}

interface ConcurrentArgs {
	mode: "concurrent" | "agent-concurrent";
	interval: number;
	concurrency: number;
	messageInterval: number;
	showMessages: boolean;
	tokensPerSecond: number;
	durationMs: number;
	endpoint: string;
}

type Args = RttArgs | ConcurrentArgs;

type WorkerHealth = "pending" | "healthy" | "warning" | "ended";

interface Sample {
	worker: number;
	key: string;
	connectMs: number;
	firstMs: number;
	secondMs: number;
	totalMs: number;
	actorId?: string;
	error?: string;
}

interface TunnelWebSocket {
	readyState: number;
	send(data: string): void;
	close(code?: number, reason?: string): void;
	addEventListener(
		type: "open" | "message" | "close" | "error",
		listener: (event: any) => void,
		options?: { once?: boolean },
	): void;
}

interface ConcurrentWorkload {
	keyPrefix: string;
	resolveActorId(handle: unknown): Promise<string>;
	openWebSocket(actorId: string, key: string): Promise<TunnelWebSocket>;
	onOpen(
		ws: TunnelWebSocket,
		worker: number,
		key: string,
		options: ConcurrentWorkerOptions,
	): () => void;
}

interface ConcurrentWorkerOptions {
	messageInterval: number;
	showMessages: boolean;
	tokensPerSecond: number;
	durationMs: number;
}

const DEFAULT_CONCURRENCY = 1_000;
const DEFAULT_CONCURRENT_INTERVAL_MS = 300;
const DEFAULT_MESSAGE_INTERVAL_MS = 1_000;
const DEFAULT_AGENT_MESSAGE_INTERVAL_MS = 30_000;
const DEFAULT_TOKENS_PER_SECOND = 20;
const DEFAULT_DURATION_MS = 5_000;
const MESSAGE_GAP_WARN_MS = 3_000;
const ACTOR_STOPPED_CLOSE_CODE = 1000;
const ACTOR_STOPPED_CLOSE_REASON = "hack_force_close";

const ANSI = {
	reset: "\x1b[0m",
	green: "\x1b[38;2;0;255;0m",
	red: "\x1b[38;2;255;0;0m",
	yellow: "\x1b[38;2;255;200;0m",
	blue: "\x1b[38;2;80;160;255m",
	dim: "\x1b[2m",
	bold: "\x1b[1m",
};

const COLOR_MIN_MS = 800;
const COLOR_MAX_MS = 2_000;

function usage(): void {
	console.error(
		"usage:\n" +
			"  tsx scripts/counter-latency.ts rtt -i <ms> <endpoint>\n" +
			"  tsx scripts/counter-latency.ts concurrent [options] <endpoint>\n" +
			"  tsx scripts/counter-latency.ts agent-concurrent [options] <endpoint>\n" +
			"\n" +
			"subcommands:\n" +
			"  rtt               spawn fresh counter actors and measure action RTTs\n" +
			"  concurrent        ramp persistent raw WebSocket tunnel-stress actors\n" +
			"  agent-concurrent  ramp persistent SQLite-backed agent actors\n" +
			"\n" +
			"  -h, --help        show usage",
	);
}

function rttUsage(): void {
	console.error(
		"usage: tsx scripts/counter-latency.ts rtt -i <ms> <endpoint>\n" +
			"  -i, --interval  gap in ms between worker starts (required)\n" +
			"  -h, --help      show usage",
	);
}

function concurrentUsage(mode: "concurrent" | "agent-concurrent"): void {
	const agentOptions =
		mode === "agent-concurrent"
			? "  --tokens-per-second      SQLite token inserts per second (default 20)\n" +
				"  --duration-ms            inference stream duration in ms (default 5000)\n"
			: "";
	const messageIntervalDefault =
		mode === "agent-concurrent" ? "30000" : "1000";
	console.error(
		`usage: tsx scripts/counter-latency.ts ${mode} [options] <endpoint>\n` +
			"  -i, --interval            ramp-up gap in ms between connections (default 300)\n" +
			"  -c, --concurrency         number of persistent connections (default 1000)\n" +
			`  --message-interval-ms     gap between client messages (default ${messageIntervalDefault})\n` +
			agentOptions +
			"  --show-messages           log all received WebSocket messages\n" +
			"  -h, --help                show usage",
	);
}

function parseRequiredMs(value: string | undefined, name: string): number {
	const parsed = Number(value);
	if (value === undefined || !Number.isFinite(parsed) || parsed < 0) {
		console.error(`${name} is required (ms, >= 0)`);
		process.exit(1);
	}
	return parsed;
}

function parseOptionalMs(
	value: string | undefined,
	name: string,
	defaultValue: number,
): number {
	if (value === undefined) return defaultValue;
	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed < 0) {
		console.error(`${name} must be ms, >= 0`);
		process.exit(1);
	}
	return parsed;
}

function parseOptionalPositiveNumber(
	value: string | undefined,
	name: string,
	defaultValue: number,
): number {
	if (value === undefined) return defaultValue;
	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed <= 0) {
		console.error(`${name} must be a positive number`);
		process.exit(1);
	}
	return parsed;
}

function parseOptionalCount(
	value: string | undefined,
	name: string,
	defaultValue: number,
): number {
	if (value === undefined) return defaultValue;
	const parsed = Number(value);
	if (!Number.isFinite(parsed) || !Number.isInteger(parsed) || parsed < 1) {
		console.error(`${name} must be an integer, >= 1`);
		process.exit(1);
	}
	return parsed;
}

function parseEndpoint(positionals: string[], usageFn: () => void): string {
	if (positionals.length === 0) {
		console.error("endpoint is required");
		usageFn();
		process.exit(1);
	}
	if (positionals.length > 1) {
		console.error(
			`unexpected positional args: ${positionals.slice(1).join(" ")}`,
		);
		usageFn();
		process.exit(1);
	}
	return positionals[0];
}

function parseRttArgs(argv: string[]): RttArgs {
	let parsed: ReturnType<
		typeof parseArgs<{
			options: {
				interval: { type: "string"; short: "i" };
				help: { type: "boolean"; short: "h" };
			};
			allowPositionals: true;
		}>
	>;
	try {
		parsed = parseArgs({
			args: argv,
			options: {
				interval: { type: "string", short: "i" },
				help: { type: "boolean", short: "h" },
			},
			allowPositionals: true,
		});
	} catch (err) {
		console.error(err instanceof Error ? err.message : String(err));
		rttUsage();
		process.exit(1);
	}

	const { values, positionals } = parsed;
	if (values.help) {
		rttUsage();
		process.exit(0);
	}

	return {
		mode: "rtt",
		interval: parseRequiredMs(values.interval, "--interval"),
		endpoint: parseEndpoint(positionals, rttUsage),
	};
}

function parseConcurrentArgs(
	mode: "concurrent" | "agent-concurrent",
	argv: string[],
): ConcurrentArgs {
	let parsed: ReturnType<
		typeof parseArgs<{
			options: {
				interval: { type: "string"; short: "i" };
				concurrency: { type: "string"; short: "c" };
				"increment-interval": { type: "string" };
				"message-interval-ms": { type: "string" };
				"show-increments": { type: "boolean" };
				"show-messages": { type: "boolean" };
				"tokens-per-second": { type: "string" };
				"duration-ms": { type: "string" };
				help: { type: "boolean"; short: "h" };
			};
			allowPositionals: true;
		}>
	>;
	try {
		parsed = parseArgs({
			args: argv,
			options: {
				interval: { type: "string", short: "i" },
				concurrency: { type: "string", short: "c" },
				"increment-interval": { type: "string" },
				"message-interval-ms": { type: "string" },
				"show-increments": { type: "boolean" },
				"show-messages": { type: "boolean" },
				"tokens-per-second": { type: "string" },
				"duration-ms": { type: "string" },
				help: { type: "boolean", short: "h" },
			},
			allowPositionals: true,
		});
	} catch (err) {
		console.error(err instanceof Error ? err.message : String(err));
		concurrentUsage(mode);
		process.exit(1);
	}

	const { values, positionals } = parsed;
	if (values.help) {
		concurrentUsage(mode);
		process.exit(0);
	}

	const defaultMessageInterval =
		mode === "agent-concurrent"
			? DEFAULT_AGENT_MESSAGE_INTERVAL_MS
			: DEFAULT_MESSAGE_INTERVAL_MS;

	return {
		mode,
		interval: parseOptionalMs(
			values.interval,
			"--interval",
			DEFAULT_CONCURRENT_INTERVAL_MS,
		),
		concurrency: parseOptionalCount(
			values.concurrency,
			"--concurrency",
			DEFAULT_CONCURRENCY,
		),
		messageInterval: parseOptionalMs(
			values["message-interval-ms"] ?? values["increment-interval"],
			"--message-interval-ms",
			defaultMessageInterval,
		),
		showMessages:
			values["show-messages"] === true ||
			values["show-increments"] === true,
		tokensPerSecond: parseOptionalPositiveNumber(
			values["tokens-per-second"],
			"--tokens-per-second",
			DEFAULT_TOKENS_PER_SECOND,
		),
		durationMs: parseOptionalMs(
			values["duration-ms"],
			"--duration-ms",
			DEFAULT_DURATION_MS,
		),
		endpoint: parseEndpoint(positionals, () => concurrentUsage(mode)),
	};
}

function parseCliArgs(argv: string[]): Args {
	const [command, ...rest] = argv;

	if (command === undefined || command === "--help" || command === "-h") {
		usage();
		process.exit(command === undefined ? 1 : 0);
	}
	if (command === "rtt") return parseRttArgs(rest);
	if (command === "concurrent" || command === "agent-concurrent") {
		return parseConcurrentArgs(command, rest);
	}

	console.error(`unknown subcommand: ${command}`);
	usage();
	process.exit(1);
}

const ARGS = parseCliArgs(process.argv.slice(2));
const BATCHES = Number(process.env.BATCHES ?? "0");
const SERIAL = ((v) => v === "1" || v === "true")(process.env.SERIAL ?? "");
const RUN_FOR_MS = parseOptionalMs(
	process.env.RUN_FOR_MS,
	"RUN_FOR_MS",
	0,
);

let concurrentWorkersStarted = 0;
let stoppingConcurrentWorkers = false;
const concurrentStats = {
	connects: 0,
	reconnects: 0,
	firstMessages: 0,
	connectErrors: 0,
	websocketErrors: 0,
	disconnects: 0,
	messageGaps: 0,
	uncleanFailuresOrDisconnects: 0,
};
const workerHealth = new Map<number, WorkerHealth>();
const workerSockets = new Set<TunnelWebSocket>();

const client = createClient<typeof registry>(ARGS.endpoint);

function setWorkerHealth(worker: number, state: WorkerHealth): void {
	workerHealth.set(worker, state);
}

function flagWorkerWarning(worker: number): void {
	if (workerHealth.get(worker) === "healthy") {
		workerHealth.set(worker, "warning");
	}
}

function countWorkerHealth(): {
	pending: number;
	healthy: number;
	warning: number;
	ended: number;
} {
	let pending = 0;
	let healthy = 0;
	let warning = 0;
	let ended = 0;
	for (const s of workerHealth.values()) {
		if (s === "pending") pending++;
		else if (s === "healthy") healthy++;
		else if (s === "warning") warning++;
		else if (s === "ended") ended++;
	}
	return { pending, healthy, warning, ended };
}

function gradientColor(ms: number): string {
	const clamped = Math.max(COLOR_MIN_MS, Math.min(COLOR_MAX_MS, ms));
	const t = (clamped - COLOR_MIN_MS) / (COLOR_MAX_MS - COLOR_MIN_MS);
	let r: number;
	let g: number;
	if (t <= 0.5) {
		r = Math.round(t * 2 * 255);
		g = 255;
	} else {
		r = 255;
		g = Math.round((1 - (t - 0.5) * 2) * 255);
	}
	return `\x1b[38;2;${r};${g};0m`;
}

function colorMs(ms: number): string {
	const fixed = ms.toFixed(0).padStart(5);
	return `${gradientColor(ms)}${fixed}ms${ANSI.reset}`;
}

function pad(s: string, n: number): string {
	return s.length >= n ? s : s + " ".repeat(n - s.length);
}

function formatActor(actorId: string | undefined): string {
	return actorId ? ` actor=${actorId}` : "";
}

function requireActorId(connection: { actorId?: string }): string {
	if (!connection.actorId) {
		throw new Error("connection actorId missing after connect");
	}
	return connection.actorId;
}

function logPrefix(_worker: number): string {
	const ts = new Date().toISOString();
	if (ARGS.mode === "rtt") {
		return `${ANSI.dim}${ts}${ANSI.reset}`;
	}

	const { pending, healthy, warning, ended } = countWorkerHealth();
	const width = String(ARGS.concurrency).length;
	const padNumber = (n: number) => String(n).padStart(width);
	const concurrencyPart = `c=${padNumber(concurrentWorkersStarted)}/${ARGS.concurrency}`;
	const pendingPart = `${ANSI.blue}${padNumber(pending)}${ANSI.reset}`;
	const healthyPart = `${ANSI.green}${padNumber(healthy)}${ANSI.reset}`;
	const warningPart = `${ANSI.yellow}${padNumber(warning)}${ANSI.reset}`;
	const endedPart = `${ANSI.red}${padNumber(ended)}${ANSI.reset}`;
	const statusPart = `s=${pendingPart}/${healthyPart}/${warningPart}/${endedPart}`;
	return `${ANSI.dim}${ts}${ANSI.reset} [${concurrencyPart} ${statusPart}]`;
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function runRttWorker(worker: number): Promise<Sample> {
	const key = `cl-${worker}-${Date.now().toString(36)}`;

	const t0 = performance.now();
	try {
		const handle = client.counter.getOrCreate([key]);
		const connection = handle.connect();

		await connection.noop();
		const tConnect = performance.now();

		await connection.increment(1);
		const tFirst = performance.now();

		await connection.increment(1);
		const tSecond = performance.now();

		const actorId = requireActorId(connection as { actorId?: string });
		void connection.dispose().catch(() => {});

		return {
			worker,
			key,
			connectMs: tConnect - t0,
			firstMs: tFirst - tConnect,
			secondMs: tSecond - tFirst,
			totalMs: tSecond - t0,
			actorId,
		};
	} catch (err) {
		const tEnd = performance.now();
		return {
			worker,
			key,
			connectMs: 0,
			firstMs: 0,
			secondMs: 0,
			totalMs: tEnd - t0,
			error: err instanceof Error ? err.message : String(err),
		};
	}
}

function printRttSample(s: Sample): void {
	const prefix = logPrefix(s.worker);
	if (s.error) {
		console.log(
			`${prefix} ${pad(s.key, 32)} ${ANSI.red}ERROR ${s.error}${ANSI.reset} (${colorMs(s.totalMs)})`,
		);
		return;
	}
	console.log(
		`${prefix} ${pad(s.key, 32)}${formatActor(s.actorId)} connect=${colorMs(s.connectMs)} first=${colorMs(s.firstMs)} second=${colorMs(s.secondMs)} total=${colorMs(s.totalMs)}`,
	);
}

function logConnect(
	worker: number,
	key: string,
	actorId: string | undefined,
	connectMs: number,
	reconnect: boolean,
): void {
	concurrentStats.connects += 1;
	if (reconnect) concurrentStats.reconnects += 1;
	setWorkerHealth(worker, "healthy");
	const prefix = logPrefix(worker);
	const label = reconnect ? "reconnect" : "connect";
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} ${label}=${colorMs(connectMs)}`,
	);
}

function logFirstMessage(
	worker: number,
	key: string,
	actorId: string | undefined,
	firstMessageMs: number,
): void {
	concurrentStats.firstMessages += 1;
	const prefix = logPrefix(worker);
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} first-message=${colorMs(firstMessageMs)}`,
	);
}

function logDisconnect(
	worker: number,
	key: string,
	actorId: string | undefined,
	reason: string,
	unclean = true,
): void {
	concurrentStats.disconnects += 1;
	if (unclean) {
		concurrentStats.uncleanFailuresOrDisconnects += 1;
	}
	setWorkerHealth(worker, "ended");
	const prefix = logPrefix(worker);
	const label = unclean ? `${ANSI.red}DISCONNECT` : `${ANSI.dim}disconnect`;
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} ${label} ${reason}${ANSI.reset}`,
	);
}

function logReconnect(
	worker: number,
	key: string,
	actorId: string | undefined,
	code: number,
	reason: string,
): void {
	setWorkerHealth(worker, "pending");
	const prefix = logPrefix(worker);
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} actor-stopped reconnect code=${code} reason=${reason}`,
	);
}

function logMessageGap(
	worker: number,
	key: string,
	actorId: string | undefined,
	gapMs: number,
): void {
	concurrentStats.messageGaps += 1;
	concurrentStats.uncleanFailuresOrDisconnects += 1;
	flagWorkerWarning(worker);
	const prefix = logPrefix(worker);
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} ${ANSI.red}MESSAGE-GAP ${colorMs(gapMs)}${ANSI.reset}`,
	);
}

function logConnectError(
	worker: number,
	key: string,
	actorId: string | undefined,
	elapsedMs: number,
	reason: string,
): void {
	concurrentStats.connectErrors += 1;
	concurrentStats.uncleanFailuresOrDisconnects += 1;
	setWorkerHealth(worker, "ended");
	const prefix = logPrefix(worker);
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} ${ANSI.red}CONNECT-ERROR ${reason}${ANSI.reset} (${colorMs(elapsedMs)})`,
	);
}

function logWebSocketError(
	worker: number,
	key: string,
	actorId: string | undefined,
): void {
	concurrentStats.websocketErrors += 1;
	concurrentStats.uncleanFailuresOrDisconnects += 1;
	flagWorkerWarning(worker);
	const prefix = logPrefix(worker);
	console.log(
		`${prefix} ${pad(key, 32)}${formatActor(actorId)} ${ANSI.red}WEBSOCKET-ERROR${ANSI.reset}`,
	);
}

function printConcurrentSummary(reason: string): void {
	if (ARGS.mode === "rtt") return;

	const { pending, healthy, warning, ended } = countWorkerHealth();
	console.log(
		`${ANSI.bold}counter-latency summary${ANSI.reset} reason=${reason} c=${concurrentWorkersStarted}/${ARGS.concurrency} s=${ANSI.blue}${pending}${ANSI.reset}/${ANSI.green}${healthy}${ANSI.reset}/${ANSI.yellow}${warning}${ANSI.reset}/${ANSI.red}${ended}${ANSI.reset} disconnects=${concurrentStats.disconnects} connect-errors=${concurrentStats.connectErrors} websocket-errors=${concurrentStats.websocketErrors} message-gaps=${concurrentStats.messageGaps} connects=${concurrentStats.connects} reconnects=${concurrentStats.reconnects} first-messages=${concurrentStats.firstMessages}`,
	);
}

function closeConcurrentWorkers(): void {
	stoppingConcurrentWorkers = true;
	for (const ws of workerSockets) {
		if (ws.readyState <= 1) {
			ws.close(1000, "counter-latency complete");
		}
	}
}

process.once("SIGINT", () => {
	closeConcurrentWorkers();
	printConcurrentSummary("sigint");
	process.exit(130);
});

process.once("SIGTERM", () => {
	closeConcurrentWorkers();
	printConcurrentSummary("sigterm");
	process.exit(143);
});

async function waitForOpen(ws: TunnelWebSocket): Promise<void> {
	if (ws.readyState === 1) return;

	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener("error", () => reject(new Error("websocket error")), {
			once: true,
		});
		ws.addEventListener(
			"close",
			(event) =>
				reject(
					new Error(
						`websocket closed before open code=${event.code} reason=${event.reason}`,
					),
				),
			{ once: true },
		);
	});
}

function eventDataToString(data: unknown): string {
	if (typeof data === "string") return data;
	if (data instanceof ArrayBuffer) return `<binary ${data.byteLength} bytes>`;
	if (ArrayBuffer.isView(data)) return `<binary ${data.byteLength} bytes>`;
	return String(data);
}

function isActorStoppedClose(event: { code: number; reason?: string }): boolean {
	return (
		event.code === ACTOR_STOPPED_CLOSE_CODE &&
		event.reason === ACTOR_STOPPED_CLOSE_REASON
	);
}

function makeTunnelStressWorkload(): ConcurrentWorkload {
	return {
		keyPrefix: "cl-t",
		async resolveActorId(handle: unknown) {
			return await (handle as ReturnType<typeof client.tunnelStress.getOrCreate>).resolve();
		},
		async openWebSocket(actorId: string, key: string) {
			const handle = actorId
				? client.tunnelStress.getForId(actorId)
				: client.tunnelStress.getOrCreate([key]);
			return (await handle.webSocket()) as TunnelWebSocket;
		},
		onOpen(ws, _worker, _key, options) {
			let sequence = 0;
			const interval = setInterval(() => {
				if (ws.readyState !== 1) return;
				sequence += 1;
				ws.send(
					JSON.stringify({
						sequence,
						timestamp: Date.now(),
					}),
				);
			}, options.messageInterval);
			return () => clearInterval(interval);
		},
	};
}

function makeAgentWorkload(): ConcurrentWorkload {
	return {
		keyPrefix: "cl-a",
		async resolveActorId(handle: unknown) {
			return await (handle as ReturnType<typeof client.loadTestAgent.getOrCreate>).resolve();
		},
		async openWebSocket(actorId: string, key: string) {
			const handle = actorId
				? client.loadTestAgent.getForId(actorId)
				: client.loadTestAgent.getOrCreate([key]);
			return (await handle.webSocket()) as TunnelWebSocket;
		},
		onOpen(ws, worker, _key, options) {
			let sequence = 0;
			const sendInference = () => {
				if (ws.readyState !== 1) return;
				sequence += 1;
				ws.send(
					JSON.stringify({
						type: "inference",
						requestId: `agent-${worker}-${Date.now().toString(36)}-${sequence}`,
						tokensPerSecond: options.tokensPerSecond,
						durationMs: options.durationMs,
					}),
				);
			};

			sendInference();
			const interval = setInterval(sendInference, options.messageInterval);
			return () => clearInterval(interval);
		},
	};
}

async function runConcurrentWorker(
	worker: number,
	workload: ConcurrentWorkload,
	options: ConcurrentWorkerOptions,
): Promise<void> {
	const key = `${workload.keyPrefix}-${worker}-${Date.now().toString(36)}`;
	let actorId: string | undefined;
	let reconnect = false;

	while (!stoppingConcurrentWorkers) {
		const t0 = performance.now();
		let cleanup: (() => void) | undefined;
		let sawWebSocketError = false;

		try {
			if (!actorId) {
				const handle =
					ARGS.mode === "agent-concurrent"
						? client.loadTestAgent.getOrCreate([key])
						: client.tunnelStress.getOrCreate([key]);
				actorId = await workload.resolveActorId(handle);
			}

			const ws = await workload.openWebSocket(actorId, key);
			workerSockets.add(ws);
			await waitForOpen(ws);
			const connectMs = performance.now() - t0;
			logConnect(worker, key, actorId, connectMs, reconnect);
			reconnect = false;

			await new Promise<void>((resolve) => {
				let firstMessageLogged = false;
				let lastMessageAt = 0;
				let settled = false;
				const settle = () => {
					if (settled) return;
					settled = true;
					resolve();
				};

				cleanup = workload.onOpen(ws, worker, key, options);

				ws.addEventListener("message", (event) => {
					const now = performance.now();
					if (!firstMessageLogged) {
						firstMessageLogged = true;
						logFirstMessage(worker, key, actorId, now - t0);
					} else if (
						lastMessageAt > 0 &&
						now - lastMessageAt > MESSAGE_GAP_WARN_MS
					) {
						logMessageGap(worker, key, actorId, now - lastMessageAt);
					}
					lastMessageAt = now;
					if (options.showMessages) {
						const prefix = logPrefix(worker);
						console.log(
							`${prefix} ${pad(key, 32)}${formatActor(actorId)} message=${eventDataToString(event.data)}`,
						);
					}
				});
				ws.addEventListener(
					"close",
					(event) => {
						if (settled) return;
						workerSockets.delete(ws);
						cleanup?.();
						if (
							!stoppingConcurrentWorkers &&
							!sawWebSocketError &&
							isActorStoppedClose(event)
						) {
							logReconnect(worker, key, actorId, event.code, event.reason);
							reconnect = true;
						} else {
							logDisconnect(
								worker,
								key,
								actorId,
								`code=${event.code} reason=${event.reason}`,
								!stoppingConcurrentWorkers,
							);
						}
						settle();
					},
					{ once: true },
				);
				ws.addEventListener("error", () => {
					sawWebSocketError = true;
					logWebSocketError(worker, key, actorId);
					workerSockets.delete(ws);
					cleanup?.();
					settle();
				});
			});
			if (sawWebSocketError) {
				setWorkerHealth(worker, "ended");
			}
		} catch (err) {
			const elapsed = performance.now() - t0;
			logConnectError(
				worker,
				key,
				actorId,
				elapsed,
				err instanceof Error ? err.message : String(err),
			);
			break;
		} finally {
			cleanup?.();
		}

		if (!reconnect) break;
	}
}

async function runRttMode(): Promise<void> {
	let workerId = 0;
	const inflight: Promise<void>[] = [];

	while (BATCHES === 0 || workerId < BATCHES) {
		workerId++;
		const id = workerId;
		if (SERIAL) {
			const sample = await runRttWorker(id);
			printRttSample(sample);
		} else {
			inflight.push(runRttWorker(id).then((s) => printRttSample(s)));
		}
		if (BATCHES === 0 || workerId < BATCHES) {
			await sleep(ARGS.interval);
		}
	}

	await Promise.all(inflight);
}

async function runConcurrentMode(): Promise<void> {
	if (ARGS.mode === "rtt") {
		throw new Error("concurrent mode called with rtt args");
	}
	const { concurrency, messageInterval, showMessages } = ARGS;
	const workload =
		ARGS.mode === "agent-concurrent"
			? makeAgentWorkload()
			: makeTunnelStressWorkload();
	const workers: Promise<void>[] = [];

	let stopTimer: ReturnType<typeof setTimeout> | undefined;
	if (RUN_FOR_MS > 0) {
		stopTimer = setTimeout(() => {
			closeConcurrentWorkers();
		}, RUN_FOR_MS);
	}

	try {
		for (let i = 0; i < concurrency; i++) {
			const id = i + 1;
			concurrentWorkersStarted = id;
			setWorkerHealth(id, "pending");
			workers.push(
				runConcurrentWorker(id, workload, {
					messageInterval,
					showMessages,
					tokensPerSecond: ARGS.tokensPerSecond,
					durationMs: ARGS.durationMs,
				}),
			);
			if (i < concurrency - 1) {
				await sleep(ARGS.interval);
			}
		}
		await Promise.all(workers);
	} finally {
		if (stopTimer) clearTimeout(stopTimer);
		printConcurrentSummary("complete");
	}
}

async function main(): Promise<void> {
	const url = new URL(ARGS.endpoint);
	const header = `${ANSI.bold}counter-latency${ANSI.reset} endpoint=${url.protocol}//${url.host} ns=${decodeURIComponent(url.username)} mode=${ARGS.mode} interval=${ARGS.interval}ms`;
	if (ARGS.mode === "rtt") {
		console.log(`${header} batches=${BATCHES || "∞"} serial=${SERIAL}`);
	} else {
		const agentPart =
			ARGS.mode === "agent-concurrent"
				? ` tokens-per-second=${ARGS.tokensPerSecond} duration-ms=${ARGS.durationMs}`
				: "";
		const runForPart = RUN_FOR_MS > 0 ? ` run-for-ms=${RUN_FOR_MS}` : "";
		console.log(
			`${header} concurrency=${ARGS.concurrency} message-every=${ARGS.messageInterval}ms show-messages=${ARGS.showMessages}${agentPart}${runForPart}`,
		);
	}
	console.log(
		`${ANSI.dim}gradient: ${gradientColor(COLOR_MIN_MS)}${COLOR_MIN_MS}ms${ANSI.reset}${ANSI.dim} -> ${gradientColor((COLOR_MIN_MS + COLOR_MAX_MS) / 2)}${(COLOR_MIN_MS + COLOR_MAX_MS) / 2}ms${ANSI.reset}${ANSI.dim} -> ${gradientColor(COLOR_MAX_MS)}${COLOR_MAX_MS}ms${ANSI.reset}`,
	);
	console.log();

	if (ARGS.mode === "rtt") {
		await runRttMode();
	} else {
		await runConcurrentMode();
	}
}

main()
	.catch((err) => {
		console.error("fatal:", err);
		process.exitCode = 1;
	})
	.finally(async () => {
		await client.dispose().catch(() => undefined);
	});
