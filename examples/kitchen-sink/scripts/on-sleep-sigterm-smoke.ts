// SIGTERM sleep handoff smoke test.
//
// Requires an already-running engine, usually at http://127.0.0.1:6420.
// Starts two kitchen-sink serverful envoys with raw node, SIGTERMs the first,
// and verifies the actor completes onSleep before reconnecting on the second.
//
// Usage:
//   pnpm --filter kitchen-sink smoke:on-sleep-sigterm -- \
//     --on-sleep-duration-ms 5000 \
//     --on-sleep-tick-ms 1000
//
// Useful overrides:
//   --endpoint http://127.0.0.1:6420
//   --namespace default
//   --pool sigterm-sleep-test
//   --reconnect-open-delay-ms 0

import {
	spawn,
	type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const KITCHEN_SINK_ROOT = fileURLToPath(new URL("..", import.meta.url));
installTimestampedConsole();
const CLI_ARGS = parseCliArgs(process.argv.slice(2));
const ENDPOINT = stringFromConfig(
	"endpoint",
	["SIGTERM_SLEEP_ENDPOINT", "RIVET_ENDPOINT"],
	"http://127.0.0.1:6420",
);
const NAMESPACE = stringFromConfig(
	"namespace",
	["SIGTERM_SLEEP_NAMESPACE", "RIVET_NAMESPACE"],
	"default",
);
const TOKEN = stringFromConfig(
	"token",
	["SIGTERM_SLEEP_TOKEN", "RIVET_TOKEN"],
	"dev",
);
const POOL_NAME = stringFromConfig(
	"pool",
	["SIGTERM_SLEEP_POOL", "RIVET_POOL"],
	`sigterm-sleep-${Date.now()}`,
);
const KEY = stringFromConfig(
	"key",
	["SIGTERM_SLEEP_KEY"],
	`sigterm-sleep-probe-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
);
const LABEL = stringFromConfig("label", ["SIGTERM_SLEEP_LABEL"], KEY);
const ON_SLEEP_DURATION_MS = numberFromConfig(
	"on-sleep-duration-ms",
	"SIGTERM_SLEEP_ON_SLEEP_DURATION_MS",
	5_000,
);
const ON_SLEEP_TICK_MS = numberFromConfig(
	"on-sleep-tick-ms",
	"SIGTERM_SLEEP_ON_SLEEP_TICK_MS",
	1_000,
);
const RUNNER_1_PORT = numberFromConfig(
	"runner-1-port",
	"SIGTERM_SLEEP_RUNNER_1_PORT",
	3101,
);
const RUNNER_2_PORT = numberFromConfig(
	"runner-2-port",
	"SIGTERM_SLEEP_RUNNER_2_PORT",
	3102,
);
const ENGINE_READY_TIMEOUT_MS = numberFromConfig(
	"engine-ready-timeout-ms",
	"SIGTERM_SLEEP_ENGINE_READY_TIMEOUT_MS",
	15_000,
);
const ENVOY_READY_TIMEOUT_MS = numberFromConfig(
	"envoy-ready-timeout-ms",
	"SIGTERM_SLEEP_ENVOY_READY_TIMEOUT_MS",
	60_000,
);
const RUNNER_EXIT_TIMEOUT_MS = numberFromConfig(
	"runner-exit-timeout-ms",
	"SIGTERM_SLEEP_RUNNER_EXIT_TIMEOUT_MS",
	Math.max(45_000, ON_SLEEP_DURATION_MS + 45_000),
);
const WS_OPEN_TIMEOUT_MS = numberFromConfig(
	"ws-open-timeout-ms",
	"SIGTERM_SLEEP_WS_OPEN_TIMEOUT_MS",
	15_000,
);
const WS_MESSAGE_TIMEOUT_MS = numberFromConfig(
	"ws-message-timeout-ms",
	"SIGTERM_SLEEP_WS_MESSAGE_TIMEOUT_MS",
	10_000,
);
const RECONNECT_MESSAGE_TIMEOUT_MS = numberFromConfig(
	"reconnect-message-timeout-ms",
	"SIGTERM_SLEEP_RECONNECT_MESSAGE_TIMEOUT_MS",
	5_000,
);
const RECONNECT_TIMEOUT_MS = numberFromConfig(
	"reconnect-timeout-ms",
	"SIGTERM_SLEEP_RECONNECT_TIMEOUT_MS",
	5_000,
);
const RECONNECT_OPEN_DELAY_MS = numberFromConfig(
	"reconnect-open-delay-ms",
	"SIGTERM_SLEEP_RECONNECT_OPEN_DELAY_MS",
	0,
);
const CLOSE_REASON = "actor stopped";
const CLOSE_CODE = 1000;
let currentPhase:
	| {
			title: string;
			startedAt: number;
	  }
	| undefined;
const COLOR_ENABLED = process.env.NO_COLOR === undefined && process.env.TERM !== "dumb";
const ANSI = {
	reset: "\x1b[0m",
	bold: "\x1b[1m",
	dim: "\x1b[2m",
	red: "\x1b[31m",
	green: "\x1b[32m",
	yellow: "\x1b[33m",
	blue: "\x1b[34m",
	magenta: "\x1b[35m",
	cyan: "\x1b[36m",
	gray: "\x1b[90m",
};

interface Envoy {
	envoy_key: string;
	pool_name: string;
	create_ts: number;
	last_ping_ts: number;
	stop_ts?: number | null;
}

interface EnvoysResponse {
	envoys: Envoy[];
}

interface ProofRow {
	id: number;
	event: string;
	sleep_count: number;
	detail: string | null;
	created_at: number;
}

interface Proof {
	state: {
		label: string;
		wakeCount: number;
		sleepCount: number;
		onSleepDurationMs: number;
		onSleepTickMs: number;
		connectionCount: number;
		messageCount: number;
		onSleepStartedAt: number | null;
		onSleepAsyncFinishedAt: number | null;
		onSleepFinishedAt: number | null;
		onSleepLastError: string | null;
	};
	rows: ProofRow[];
}

interface CloseInfo {
	code: number;
	reason: string;
	wasClean: boolean;
	at: number;
}

function logTimestamp(): string {
	return new Date().toISOString();
}

function color(text: string, code: string): string {
	if (!COLOR_ENABLED) return text;
	return `${code}${text}${ANSI.reset}`;
}

function colorForPrefix(prefix: string): string {
	if (prefix.startsWith("[runner:one]")) return ANSI.yellow;
	if (prefix.startsWith("[runner:two]")) return ANSI.green;
	if (prefix.startsWith("[runner:")) return ANSI.green;
	if (prefix.startsWith("[envoys]")) return ANSI.blue;
	if (prefix.startsWith("[ws:error]")) return ANSI.red;
	if (prefix.startsWith("[ws:close]")) return ANSI.yellow;
	if (prefix.startsWith("[ws:message]")) return ANSI.gray;
	if (prefix.startsWith("[ws]")) return ANSI.magenta;
	if (prefix.startsWith("[test]")) return ANSI.cyan;
	return ANSI.gray;
}

function colorizeLogText(text: string, level: "log" | "warn" | "error"): string {
	const colored = text.replace(/^(\[[^\]]+\])/, (prefix) =>
		color(prefix, level === "error" ? ANSI.red : colorForPrefix(prefix)),
	);
	return level === "warn" ? color(colored, ANSI.yellow) : colored;
}

function formatTimestamp(): string {
	return color(`[${logTimestamp()}]`, ANSI.dim + ANSI.gray);
}

function formatConsoleArgs(
	level: "log" | "warn" | "error",
	args: unknown[],
): unknown[] {
	if (typeof args[0] !== "string") return [formatTimestamp(), ...args];
	return [formatTimestamp(), colorizeLogText(args[0], level), ...args.slice(1)];
}

function formatDuration(ms: number): string {
	return `${ms}ms ${(ms / 1000).toFixed(3)}s`;
}

function logPhase(title: string): void {
	finishPhase();
	const line = "=".repeat(88);
	const code = ANSI.bold + ANSI.magenta;
	console.log(color(`[phase] ${line}`, code));
	console.log(color(`[phase] ${title.toUpperCase()}`, code));
	console.log(color(`[phase] ${line}`, code));
	currentPhase = {
		title,
		startedAt: Date.now(),
	};
}

function finishPhase(): void {
	if (!currentPhase) return;
	const durationMs = Date.now() - currentPhase.startedAt;
	const code = ANSI.bold + ANSI.green;
	console.log(
		color(
			`[phase] complete "${currentPhase.title}" duration=${formatDuration(durationMs)}`,
			code,
		),
	);
	currentPhase = undefined;
}

function installTimestampedConsole(): void {
	const originalLog = console.log.bind(console);
	const originalError = console.error.bind(console);
	const originalWarn = console.warn.bind(console);

	console.log = (...args: unknown[]) => originalLog(...formatConsoleArgs("log", args));
	console.error = (...args: unknown[]) =>
		originalError(...formatConsoleArgs("error", args));
	console.warn = (...args: unknown[]) =>
		originalWarn(...formatConsoleArgs("warn", args));
}

function parseCliArgs(args: string[]): Map<string, string> {
	const parsed = new Map<string, string>();
	for (let i = 0; i < args.length; i += 1) {
		const arg = args[i];
		if (arg === "--") continue;
		if (!arg.startsWith("--")) {
			throw new Error(`unexpected argument "${arg}". Use --name value.`);
		}

		const eqIndex = arg.indexOf("=");
		if (eqIndex !== -1) {
			const name = arg.slice(2, eqIndex);
			const value = arg.slice(eqIndex + 1);
			if (!name || value === "") {
				throw new Error(`invalid argument "${arg}". Use --name=value.`);
			}
			parsed.set(name, value);
			continue;
		}

		const name = arg.slice(2);
		const value = args[i + 1];
		if (!name || value === undefined || value.startsWith("--")) {
			throw new Error(`missing value for --${name}`);
		}
		parsed.set(name, value);
		i += 1;
	}
	return parsed;
}

function stringFromConfig(
	argName: string,
	envNames: string[],
	fallback: string,
): string {
	const arg = CLI_ARGS.get(argName);
	if (arg !== undefined) return arg;

	for (const envName of envNames) {
		const raw = process.env[envName];
		if (raw !== undefined && raw !== "") return raw;
	}

	return fallback;
}

function numberFromConfig(
	argName: string,
	envName: string,
	fallback: number,
): number {
	const raw = CLI_ARGS.get(argName) ?? process.env[envName];
	if (raw === undefined || raw === "") return fallback;
	const parsed = Number(raw);
	if (!Number.isFinite(parsed) || parsed < 0) {
		throw new Error(`--${argName} must be a finite non-negative number`);
	}
	return parsed;
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function formatError(error: unknown): string {
	if (error instanceof Error) return `${error.name}: ${error.message}`;
	return String(error);
}

function appendPath(endpoint: string, path: string): URL {
	const url = new URL(endpoint);
	const prefix = url.pathname.replace(/\/$/, "");
	url.pathname = `${prefix}${path}`;
	url.search = "";
	url.hash = "";
	return url;
}

function buildEnvoysUrl(): string {
	const url = appendPath(ENDPOINT, "/envoys");
	url.searchParams.set("namespace", NAMESPACE);
	url.searchParams.set("name", POOL_NAME);
	url.searchParams.set("limit", "100");
	return url.toString();
}

function buildWebSocketUrl(_actorId: string): string {
	const url = appendPath(
		ENDPOINT,
		`/gateway/sigtermSleepProbe/websocket`,
	);
	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
	url.searchParams.set("rvt-namespace", NAMESPACE);
	url.searchParams.set("rvt-method", "getOrCreate");
	url.searchParams.set("rvt-runner", POOL_NAME);
	url.searchParams.set("rvt-key", KEY);
	url.searchParams.set("rvt-crash-policy", "sleep");
	url.searchParams.set("rvt-skip-ready-wait", "true");
	if (TOKEN) {
		url.searchParams.set("rvt-token", TOKEN);
	}
	return url.toString();
}

function runnerEnv(port: number): NodeJS.ProcessEnv {
	const env: NodeJS.ProcessEnv = {
		...process.env,
		RIVET_KITCHEN_SINK_MODE: "serverful",
		RIVET_ENDPOINT: ENDPOINT,
		RIVET_NAMESPACE: NAMESPACE,
		RIVET_TOKEN: TOKEN,
		RIVET_POOL: POOL_NAME,
		RIVET_LOG_LEVEL: process.env.RIVET_LOG_LEVEL ?? "info",
		RIVET_LOG_TARGET: process.env.RIVET_LOG_TARGET ?? "1",
		RIVET_LOG_TIMESTAMP: process.env.RIVET_LOG_TIMESTAMP ?? "1",
		PORT: String(port),
	};
	delete env.RIVET_RUN_ENGINE;
	delete env.RIVET_SERVERLESS_URL;
	delete env.KITCHEN_SINK_SERVERLESS_URL;
	return env;
}

function startRunner(
	label: string,
	port: number,
): ChildProcessWithoutNullStreams {
	const runner = spawn(
		process.execPath,
		[
			"--experimental-strip-types",
			"--experimental-transform-types",
			"--import",
			"@rivetkit/sql-loader",
			"src/server.ts",
		],
		{
			cwd: KITCHEN_SINK_ROOT,
			env: runnerEnv(port),
			stdio: ["ignore", "pipe", "pipe"],
		},
	);

	runner.stdout.on("data", (chunk) => {
		process.stdout.write(prefixChunk(`[runner:${label}]`, chunk));
	});
	runner.stderr.on("data", (chunk) => {
		process.stderr.write(prefixChunk(`[runner:${label}]`, chunk));
	});

	console.log(`[test] started ${label} pid=${runner.pid} port=${port}`);
	return runner;
}

function prefixChunk(prefix: string, chunk: Buffer): string {
	return chunk
		.toString("utf8")
		.split(/\r?\n/)
		.map((line, index, lines) => {
			if (line === "" && index === lines.length - 1) return "";
			return `${formatTimestamp()} ${color(prefix, colorForPrefix(prefix))} ${line}`;
		})
		.join("\n");
}

async function waitForExit(
	runner: ChildProcessWithoutNullStreams,
	timeoutMs: number,
): Promise<{ code: number | null; signal: NodeJS.Signals | null }> {
	if (runner.exitCode !== null || runner.signalCode !== null) {
		return { code: runner.exitCode, signal: runner.signalCode };
	}

	const exitPromise = once(runner, "exit").then(([code, signal]) => ({
		code: code as number | null,
		signal: signal as NodeJS.Signals | null,
	}));
	const result = await Promise.race([
		exitPromise,
		sleep(timeoutMs).then(() => null),
	]);
	if (result) return result;
	throw new Error(`runner did not exit within ${timeoutMs}ms`);
}

async function stopRunner(
	runner: ChildProcessWithoutNullStreams | undefined,
	label: string,
): Promise<{ code: number | null; signal: NodeJS.Signals | null } | undefined> {
	if (!runner || runner.exitCode !== null || runner.signalCode !== null) {
		return undefined;
	}

	if (runner.pid === undefined) {
		throw new Error(`runner ${label} has no pid`);
	}

	console.log(`[test] sending SIGTERM to ${label} runner pid=${runner.pid}`);
	runner.kill("SIGTERM");

	try {
		const exit = await waitForExit(runner, RUNNER_EXIT_TIMEOUT_MS);
		console.log(
			`[test] ${label} runner exited code=${exit.code} signal=${exit.signal}`,
		);
		return exit;
	} catch (error) {
		console.error(
			`[test] ${label} runner did not exit cleanly: ${formatError(error)}`,
		);
		try {
			runner.kill("SIGKILL");
		} catch {}
		throw error;
	}
}

async function fetchEnvoys(): Promise<Envoy[]> {
	const url = buildEnvoysUrl();
	const response = await fetch(url, {
		headers: TOKEN ? { Authorization: `Bearer ${TOKEN}` } : undefined,
	});
	const body = await response.text();
	console.log(`[envoys] status=${response.status} url=${url}`);
	if (!response.ok) {
		throw new Error(`GET /envoys status=${response.status} body=${body}`);
	}
	const parsed = JSON.parse(body) as EnvoysResponse;
	return parsed.envoys.filter((envoy) => envoy.stop_ts === undefined || envoy.stop_ts === null);
}

async function validateEngine(): Promise<void> {
	const deadline = Date.now() + ENGINE_READY_TIMEOUT_MS;
	let lastError = "not attempted";
	while (Date.now() < deadline) {
		try {
			await fetchEnvoys();
			console.log(`[test] engine is reachable at ${ENDPOINT}`);
			return;
		} catch (error) {
			lastError = formatError(error);
			await sleep(500);
		}
	}
	throw new Error(`engine is not reachable at ${ENDPOINT}: ${lastError}`);
}

async function waitForEnvoyCount(
	count: number,
	timeoutMs: number,
): Promise<Envoy[]> {
	const deadline = Date.now() + timeoutMs;
	let lastEnvoys: Envoy[] = [];
	while (Date.now() < deadline) {
		lastEnvoys = await fetchEnvoys();
		const keys = lastEnvoys.map((envoy) => envoy.envoy_key).join(",");
		console.log(
			`[envoys] active=${lastEnvoys.length} expected>=${count} keys=${keys}`,
		);
		if (lastEnvoys.length >= count) return lastEnvoys;
		await sleep(500);
	}
	throw new Error(
		`timed out waiting for ${count} active envoys in pool ${POOL_NAME}; saw ${lastEnvoys.length}`,
	);
}

function client() {
	return createClient<typeof registry>({
		endpoint: ENDPOINT,
		namespace: NAMESPACE,
		token: TOKEN,
		poolName: POOL_NAME,
	});
}

async function waitForOpen(ws: WebSocket, timeoutMs: number): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;
	await new Promise<void>((resolve, reject) => {
		const timeout = setTimeout(
			() => reject(new Error(`websocket open timeout after ${timeoutMs}ms`)),
			timeoutMs,
		);
		ws.addEventListener(
			"open",
			() => {
				clearTimeout(timeout);
				resolve();
			},
			{ once: true },
		);
		ws.addEventListener(
			"error",
			() => {
				clearTimeout(timeout);
				reject(new Error("websocket error before open"));
			},
			{ once: true },
		);
		ws.addEventListener(
			"close",
			(event) => {
				clearTimeout(timeout);
				reject(
					new Error(
						`websocket closed before open code=${event.code} reason=${event.reason}`,
					),
				);
			},
			{ once: true },
		);
	});
}

async function waitForMessage(
	ws: WebSocket,
	predicate: (message: any) => boolean,
	timeoutMs: number,
	label: string,
): Promise<any> {
	return await new Promise((resolve, reject) => {
		const timeout = setTimeout(
			() => reject(new Error(`${label} message timeout after ${timeoutMs}ms`)),
			timeoutMs,
		);
		const onMessage = (event: MessageEvent) => {
			const data = typeof event.data === "string" ? event.data : String(event.data);
			console.log(`[ws:message] ${data}`);
			let parsed: any;
			try {
				parsed = JSON.parse(data);
			} catch {
				return;
			}
			if (!predicate(parsed)) return;
			cleanup();
			resolve(parsed);
		};
		const onClose = (event: CloseEvent) => {
			cleanup();
			reject(
				new Error(
					`${label} closed while waiting code=${event.code} reason=${event.reason}`,
				),
			);
		};
		const cleanup = () => {
			clearTimeout(timeout);
			ws.removeEventListener("message", onMessage);
			ws.removeEventListener("close", onClose);
		};
		ws.addEventListener("message", onMessage);
		ws.addEventListener("close", onClose, { once: true });
	});
}

async function connectAndPingPong(
	actorId: string,
	label: string,
	openTimeoutMs = WS_OPEN_TIMEOUT_MS,
	messageTimeoutMs = WS_MESSAGE_TIMEOUT_MS,
): Promise<WebSocket> {
	const wsUrl = buildWebSocketUrl(actorId);
	console.log(`[ws] ${label} connecting url=${wsUrl}`);
	const ws = new WebSocket(wsUrl, ["rivet", "rivet_encoding.json"]);
	ws.addEventListener("close", (event) => {
		console.log(
			`[ws:close] ${label} code=${event.code} reason=${event.reason} wasClean=${event.wasClean}`,
		);
	});
	ws.addEventListener("error", () => {
		console.error(`[ws:error] ${label}`);
	});

	try {
		await waitForOpen(ws, openTimeoutMs);
		console.log(`[ws] ${label} open`);
		await waitForMessage(
			ws,
			(message) => message.type === "welcome",
			messageTimeoutMs,
			`${label} welcome`,
		);
		ws.send(JSON.stringify({ type: "ping", label, timestamp: Date.now() }));
		await waitForMessage(
			ws,
			(message) => message.type === "pong",
			messageTimeoutMs,
			`${label} pong`,
		);
		ws.addEventListener("message", (event) => {
			const data = typeof event.data === "string" ? event.data : String(event.data);
			console.log(`[ws:message] ${label} ${data}`);
		});
		console.log(`[ws] ${label} ping pong ok`);
		return ws;
	} catch (error) {
		if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
			ws.close(1000, `${label} retry`);
		}
		throw error;
	}
}

async function reconnectAndPingPong(
	actorId: string,
	timeoutMs: number,
): Promise<WebSocket> {
	console.log(
		`[ws] reconnect strict timeoutMs=${timeoutMs} messageTimeoutMs=${RECONNECT_MESSAGE_TIMEOUT_MS} openDelayMs=${RECONNECT_OPEN_DELAY_MS}`,
	);
	if (RECONNECT_OPEN_DELAY_MS > 0) {
		console.log(`[ws] reconnect waiting before open delayMs=${RECONNECT_OPEN_DELAY_MS}`);
		await sleep(RECONNECT_OPEN_DELAY_MS);
	}
	return await connectAndPingPong(
		actorId,
		"reconnect",
		Math.min(WS_OPEN_TIMEOUT_MS, timeoutMs),
		Math.min(RECONNECT_MESSAGE_TIMEOUT_MS, timeoutMs),
	);
}

function waitForClose(ws: WebSocket, timeoutMs: number): Promise<CloseInfo> {
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(
			() => reject(new Error(`websocket close timeout after ${timeoutMs}ms`)),
			timeoutMs,
		);
		ws.addEventListener(
			"close",
			(event) => {
				clearTimeout(timeout);
				resolve({
					code: event.code,
					reason: event.reason,
					wasClean: event.wasClean,
					at: Date.now(),
				});
			},
			{ once: true },
		);
	});
}

function assertClose(close: CloseInfo, sigtermAt: number): void {
	const elapsedMs = close.at - sigtermAt;
	if (close.code !== CLOSE_CODE || close.reason !== CLOSE_REASON) {
		throw new Error(
			`expected close code=${CLOSE_CODE} reason=${CLOSE_REASON}; got code=${close.code} reason=${close.reason}`,
		);
	}
	if (elapsedMs < ON_SLEEP_DURATION_MS) {
		throw new Error(
			`websocket closed too early: ${elapsedMs}ms < ${ON_SLEEP_DURATION_MS}ms`,
		);
	}
	if (elapsedMs > ON_SLEEP_DURATION_MS + 10_000) {
		throw new Error(
			`websocket closed too late: ${elapsedMs}ms > ${ON_SLEEP_DURATION_MS + 10_000}ms`,
		);
	}
	console.log(
		`[test] shutdown close matched code=${close.code} reason=${close.reason} elapsedMs=${elapsedMs}`,
	);
}

function assertProof(proof: Proof): void {
	const events = proof.rows.map((row) => row.event);
	const start = proof.rows.find((row) => row.event === "on-sleep-start");
	const afterAwait = proof.rows.find(
		(row) => row.event === "on-sleep-after-await",
	);
	const finish = proof.rows.find((row) => row.event === "on-sleep-finish");
	const ticks = proof.rows.filter((row) => row.event === "on-sleep-tick");

	if (proof.state.sleepCount < 1) {
		throw new Error(`expected sleepCount >= 1, got ${proof.state.sleepCount}`);
	}
	if (proof.state.onSleepLastError !== null) {
		throw new Error(`onSleep error: ${proof.state.onSleepLastError}`);
	}
	if (!start || !afterAwait || !finish) {
		throw new Error(
			`missing onSleep proof rows. saw events=${events.join(",")}`,
		);
	}

	const elapsedMs = afterAwait.created_at - start.created_at;
	if (elapsedMs < ON_SLEEP_DURATION_MS) {
		throw new Error(
			`onSleep proof delay too short: ${elapsedMs}ms < ${ON_SLEEP_DURATION_MS}ms`,
		);
	}
	if (finish.created_at < afterAwait.created_at) {
		throw new Error("on-sleep-finish row was written before async row");
	}

	const expectedTicks = Math.ceil(ON_SLEEP_DURATION_MS / ON_SLEEP_TICK_MS);
	if (ticks.length < expectedTicks) {
		throw new Error(
			`expected at least ${expectedTicks} on-sleep-tick rows, got ${ticks.length}`,
		);
	}
}

async function main(): Promise<void> {
	if (ON_SLEEP_DURATION_MS <= 0) {
		throw new Error("SIGTERM_SLEEP_ON_SLEEP_DURATION_MS must be positive");
	}
	if (ON_SLEEP_TICK_MS <= 0) {
		throw new Error("SIGTERM_SLEEP_ON_SLEEP_TICK_MS must be positive");
	}
	if (RECONNECT_OPEN_DELAY_MS < 0) {
		throw new Error("SIGTERM_SLEEP_RECONNECT_OPEN_DELAY_MS must be non-negative");
	}

	console.log(
		`[test] endpoint=${ENDPOINT} namespace=${NAMESPACE} pool=${POOL_NAME} key=${KEY} durationMs=${ON_SLEEP_DURATION_MS} tickMs=${ON_SLEEP_TICK_MS} reconnectOpenDelayMs=${RECONNECT_OPEN_DELAY_MS}`,
	);

	let runner1: ChildProcessWithoutNullStreams | undefined;
	let runner2: ChildProcessWithoutNullStreams | undefined;
	let ws1: WebSocket | undefined;
	let ws2: WebSocket | undefined;

	try {
		logPhase("1. Validate engine");
		await validateEngine();

		logPhase("2. Start kitchen-sink runner one");
		runner1 = startRunner("one", RUNNER_1_PORT);
		await waitForEnvoyCount(1, ENVOY_READY_TIMEOUT_MS);

		logPhase("3. Create and prepare actor");
		const firstClient = client();
		const handle = firstClient.sigtermSleepProbe.getOrCreate([KEY]);
		const actorId = await handle.resolve();
		console.log(`[test] actorId=${actorId}`);
		const prepared = await handle.prepare(
			LABEL,
			ON_SLEEP_DURATION_MS,
			ON_SLEEP_TICK_MS,
		);
		console.log(`[test] prepared ${JSON.stringify(prepared)}`);

		logPhase("4. Initial websocket ping pong");
		ws1 = await connectAndPingPong(actorId, "initial");

		logPhase("5. Start kitchen-sink runner two");
		runner2 = startRunner("two", RUNNER_2_PORT);
		await waitForEnvoyCount(2, ENVOY_READY_TIMEOUT_MS);

		logPhase("6. SIGTERM runner one and wait for onSleep close");
		const closePromise = waitForClose(
			ws1,
			ON_SLEEP_DURATION_MS + RUNNER_EXIT_TIMEOUT_MS,
		);
		const sigtermAt = Date.now();
		const runner1ExitPromise = stopRunner(runner1, "one");
		runner1ExitPromise.catch(() => undefined);
		runner1 = undefined;
		const close = await closePromise;
		assertClose(close, sigtermAt);

		logPhase("7. Reconnect through runner two");
		ws2 = await reconnectAndPingPong(actorId, RECONNECT_TIMEOUT_MS);

		const runner1Exit = await runner1ExitPromise;
		console.log(`[test] first runner shutdown ${JSON.stringify(runner1Exit)}`);

		logPhase("8. Verify database proof");
		const proof = (await client().sigtermSleepProbe
			.getOrCreate([KEY])
			.getProof()) as Proof;
		assertProof(proof);

		console.log("[test] proof rows:");
		for (const row of proof.rows) {
			console.log(
				`[test]   #${row.id} ${row.event} sleep=${row.sleep_count} detail=${row.detail ?? ""} at=${new Date(row.created_at).toISOString()}`,
			);
		}
		console.log(`[test] proof state ${JSON.stringify(proof.state)}`);
		console.log("[test] PASS onSleep completed during SIGTERM and actor reconnected on the second kitchen-sink envoy");
	} finally {
		logPhase("9. Cleanup");
		if (ws2 && ws2.readyState === WebSocket.OPEN) ws2.close(1000, "smoke done");
		if (ws1 && ws1.readyState === WebSocket.OPEN) ws1.close(1000, "smoke done");
		await stopRunner(runner2, "two").catch((error) => {
			console.error(`[test] runner two cleanup failed: ${formatError(error)}`);
		});
		await stopRunner(runner1, "one").catch((error) => {
			console.error(`[test] runner one cleanup failed: ${formatError(error)}`);
		});
		finishPhase();
	}
}

main().then(() => process.exit(0));
