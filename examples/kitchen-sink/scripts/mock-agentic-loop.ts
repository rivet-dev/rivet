#!/usr/bin/env -S pnpm exec tsx

import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import {
	existsSync,
	mkdtempSync,
	readdirSync,
	readFileSync,
	rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const ENDPOINT =
	process.env.RIVET_ENDPOINT ??
	process.env.VITE_RIVET_ENDPOINT ??
	"http://127.0.0.1:6420";
const START_SERVER =
	process.env.MOCK_AGENTIC_START_SERVER === "1" ||
	(process.env.MOCK_AGENTIC_START_SERVER !== "0" &&
		!process.env.RIVET_SERVERLESS_URL);
const DEFAULT_SERVERLESS_URL = "http://127.0.0.1:3000/api/rivet";
const SERVERLESS_URL =
	process.env.RIVET_SERVERLESS_URL ??
	(START_SERVER ? DEFAULT_SERVERLESS_URL : undefined);
const SERVER_READY_TIMEOUT_MS = numberFromEnv(
	"MOCK_AGENTIC_SERVER_READY_TIMEOUT_MS",
	60_000,
);
const SERVER_LOGS = process.env.MOCK_AGENTIC_SERVER_LOGS === "1";
const NAMESPACE =
	process.env.MOCK_AGENTIC_NAMESPACE ??
	process.env.RIVET_NAMESPACE ??
	"default";
const TOKEN =
	process.env.MOCK_AGENTIC_TOKEN ?? process.env.RIVET_TOKEN ?? "dev";
const POOL_NAME =
	process.env.MOCK_AGENTIC_POOL ?? process.env.RIVET_POOL ?? "default";
const KEY_PREFIX = process.env.MOCK_AGENTIC_KEY_PREFIX ?? "mock-agentic-loop";
const DURATION_MS = numberFromEnv("MOCK_AGENTIC_DURATION_MS", 180_000);
const INFERENCE_MIN_SECONDS = numberFromEnv(
	"MOCK_AGENTIC_INFERENCE_MIN_SECONDS",
	15,
);
const INFERENCE_MAX_SECONDS = numberFromEnv(
	"MOCK_AGENTIC_INFERENCE_MAX_SECONDS",
	60,
);
const JITTER_MIN_MS = numberFromEnv("MOCK_AGENTIC_JITTER_MIN_MS", 0);
const JITTER_MAX_MS = numberFromEnv("MOCK_AGENTIC_JITTER_MAX_MS", 15_000);
const PROGRESS_MARGIN_MS = numberFromEnv(
	"MOCK_AGENTIC_PROGRESS_MARGIN_MS",
	5_000,
);
const OPEN_TIMEOUT_MS = numberFromEnv("MOCK_AGENTIC_OPEN_TIMEOUT_MS", 35_000);
const RECONNECT_DELAY_MS = numberFromEnv(
	"MOCK_AGENTIC_RECONNECT_DELAY_MS",
	500,
);
const MAX_RECONNECT_MS = numberFromEnv(
	"MOCK_AGENTIC_MAX_RECONNECT_MS",
	30_000,
);
const DEFAULT_ON_SLEEP_DELAY_MS = 0;
const ON_SLEEP_DELAY_MS = numberFromEnv(
	"MOCK_AGENTIC_ON_SLEEP_DELAY_MS",
	DEFAULT_ON_SLEEP_DELAY_MS,
);
const SLEEP_CLOSE_TIMEOUT_MS = numberFromEnv(
	"MOCK_AGENTIC_SLEEP_CLOSE_TIMEOUT_MS",
	ON_SLEEP_DELAY_MS + 30_000,
);
const PROBE_INTERVAL_MS = numberFromEnv(
	"MOCK_AGENTIC_PROBE_INTERVAL_MS",
	1_000,
);
const PROBE_TIMEOUT_MS = numberFromEnv("MOCK_AGENTIC_PROBE_TIMEOUT_MS", 35_000);
const BYPASS_INTERVAL_MS = numberFromEnv(
	"MOCK_AGENTIC_BYPASS_INTERVAL_MS",
	1_000,
);
const BYPASS_TIMEOUT_MS = numberFromEnv(
	"MOCK_AGENTIC_BYPASS_TIMEOUT_MS",
	10_000,
);
const EXPECTED_PROBE_CLOSE_CODE = numberFromEnv(
	"MOCK_AGENTIC_EXPECTED_PROBE_CLOSE_CODE",
	1011,
);
const EXPECTED_PROBE_CLOSE_REASON_PREFIX =
	process.env.MOCK_AGENTIC_EXPECTED_PROBE_CLOSE_REASON_PREFIX ??
	"actor.stopping";
const SLEEP_INTERVAL_MS = 120_000;
const KITCHEN_SINK_DIR = fileURLToPath(new URL("..", import.meta.url));
const REPO_ROOT = join(KITCHEN_SINK_DIR, "../..");

type ServerMessage =
	| { type: "hello"; connectionId: string; timestamp: number }
	| {
			type: "history";
			totalRows: number;
			entries: HistoryEntry[];
			timestamp: number;
	  }
	| {
			type: "pong";
			probeId: string;
			sleepStarted: boolean;
			sleepStartedAt: number | null;
			timestamp: number;
	  }
	| { type: "started"; requestId: string; seconds: number; timestamp: number }
	| {
			type: "progress";
			requestId: string;
			idx: number;
			seconds: number;
			createdAt: number;
	  }
	| {
			type: "done";
			requestId: string;
			seconds: number;
			timestamp: number;
			verification: Verification;
	  }
	| Verification
	| { type: "error"; message: string; timestamp: number };

type Verification = {
	type: "verified";
	requestId: string;
	expectedSeconds: number;
	count: number;
	contiguous: boolean;
	missing: number[];
	indexes: number[];
	ok: boolean;
};

type ActionVerification = {
	requestId: string;
	expectedSeconds: number;
	count: number;
	indexes: number[];
};

type HistoryEntry = {
	request_id: string;
	idx: number;
	created_at: number;
};

type RequestExpectation = {
	requestId: string;
	seconds: number;
};

type AllVerification = {
	type: "verifiedAll";
	expectedRequests: number;
	expectedTotalRows: number;
	totalRows: number;
	unexpectedRequestIds: string[];
	requests: Verification[];
	ok: boolean;
};

type ActionVerifier = {
	verify: (
		requestId: string,
		expectedSeconds: number,
	) => Promise<ActionVerification>;
	verifyAll: (expectedRequests: RequestExpectation[]) => Promise<AllVerification>;
};

type Waiter = {
	accept: (message: ServerMessage) => boolean;
	resolve: (message: ServerMessage) => void;
	reject: (error: Error) => void;
	timeout: NodeJS.Timeout;
};

type CloseObservation = {
	code: number;
	reason: string;
	timestamp: number;
	phase: string;
};

type SleepStats = {
	posts: number;
	errors: number;
	postTimes: number[];
};

type ProbeStats = {
	attempts: number;
	successes: number;
	expectedCloses: number;
	unexpectedCloses: CloseObservation[];
	timeouts: number;
	errors: string[];
	expectedCloseSamples: CloseObservation[];
};

type BypassPhase = "beforeSleep" | "afterSleep";

type BypassObservation = {
	phase: BypassPhase;
	message: string;
};

type BypassStats = {
	attempts: number;
	beforeSleepAttempts: number;
	afterSleepAttempts: number;
	httpSuccesses: number;
	beforeSleepHttpSuccesses: number;
	afterSleepHttpSuccesses: number;
	beforeSleepHttpUnexpectedSleepStarted: number;
	afterSleepHttpSleepStarted: number;
	webSocketSuccesses: number;
	beforeSleepWebSocketSuccesses: number;
	afterSleepWebSocketSuccesses: number;
	beforeSleepWebSocketUnexpectedSleepStarted: number;
	afterSleepWebSocketSleepStarted: number;
	timeouts: BypassObservation[];
	errors: BypassObservation[];
};

type BypassHandle = {
	fetch: (
		input: string,
		init?: RequestInit & {
			gateway?: {
				skipReadyWait?: boolean;
			};
		},
	) => Promise<Response>;
	webSocket: (
		path?: string,
		protocols?: string | string[],
		options?: {
			gateway?: {
				skipReadyWait?: boolean;
			};
		},
	) => Promise<WebSocket>;
};

type LocalKitchenSinkServer = {
	child: ChildProcessWithoutNullStreams;
	dbRoot: string;
	enginePort: number;
	runId: string;
	serverPort: number;
	logs: string[];
};

function numberFromEnv(name: string, fallback: number): number {
	const value = process.env[name];
	if (value === undefined || value === "") return fallback;

	const parsed = Number(value);
	if (!Number.isFinite(parsed)) {
		throw new Error(`${name} must be a finite number`);
	}

	return parsed;
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function withTimeout<T>(
	promise: Promise<T>,
	label: string,
	timeoutMs: number,
): Promise<T> {
	let timeoutHandle: NodeJS.Timeout | undefined;
	try {
		return await Promise.race([
			promise,
			new Promise<never>((_resolve, reject) => {
				timeoutHandle = setTimeout(
					() => reject(new Error(`${label} timed out after ${timeoutMs}ms`)),
					timeoutMs,
				);
			}),
		]);
	} finally {
		if (timeoutHandle) clearTimeout(timeoutHandle);
	}
}

function portFromUrl(urlString: string): number {
	const url = new URL(urlString);
	if (url.port) return Number(url.port);
	if (url.protocol === "https:" || url.protocol === "wss:") return 443;
	return 80;
}

async function listenerPids(port: number): Promise<number[]> {
	return await new Promise((resolve, reject) => {
		const child = spawn(
			"lsof",
			[`-tiTCP:${port}`, "-sTCP:LISTEN", "-Pn"],
			{
				stdio: ["ignore", "pipe", "pipe"],
			},
		);
		let stdout = "";
		let stderr = "";
		child.stdout.on("data", (chunk) => {
			stdout += chunk.toString();
		});
		child.stderr.on("data", (chunk) => {
			stderr += chunk.toString();
		});
		child.on("error", reject);
		child.on("exit", (code) => {
			if (code === 0 || code === 1) {
				resolve(
					stdout
						.split(/\s+/)
						.filter(Boolean)
						.map((pid) => Number(pid))
						.filter((pid) => Number.isInteger(pid) && pid > 0),
				);
			} else {
				reject(new Error(`lsof failed for port ${port}: ${stderr}`));
			}
		});
	});
}

async function assertPortAvailable(port: number, label: string) {
	const pids = await listenerPids(port);
	if (pids.length > 0) {
		throw new Error(
			`${label} port ${port} is already in use by pid(s): ${pids.join(", ")}`,
		);
	}
}

async function stopListeners(port: number, label: string) {
	let pids = await listenerPids(port);
	if (pids.length === 0) return;

	console.log(`[server] stopping ${label} listener port=${port} pids=${pids.join(",")}`);
	for (const pid of pids) {
		try {
			process.kill(pid, "SIGTERM");
		} catch {
			// The process may have exited between lsof and kill.
		}
	}
	await sleep(2_000);

	pids = await listenerPids(port);
	for (const pid of pids) {
		try {
			process.kill(pid, "SIGKILL");
		} catch {
			// The process may have exited between lsof and kill.
		}
	}
}

function pidsWithEnvValue(name: string, value: string): number[] {
	const pids: number[] = [];
	for (const entry of readdirSync("/proc")) {
		if (!/^\d+$/.test(entry)) continue;

		const pid = Number(entry);
		if (pid === process.pid) continue;

		try {
			const env = readFileSync(`/proc/${entry}/environ`, "utf8");
			if (env.split("\0").includes(`${name}=${value}`)) {
				pids.push(pid);
			}
		} catch {
			// Processes can exit or deny access while scanning.
		}
	}
	return pids;
}

async function stopProcessesWithEnvValue(name: string, value: string) {
	let pids = pidsWithEnvValue(name, value);
	if (pids.length === 0) return;

	console.log(`[server] stopping tagged processes pids=${pids.join(",")}`);
	for (const pid of pids) {
		try {
			process.kill(pid, "SIGTERM");
		} catch {
			// The process may have exited between scan and kill.
		}
	}
	await sleep(2_000);

	pids = pidsWithEnvValue(name, value);
	for (const pid of pids) {
		try {
			process.kill(pid, "SIGKILL");
		} catch {
			// The process may have exited between scan and kill.
		}
	}
}

function randomInteger(min: number, max: number): number {
	if (max < min) {
		throw new Error("max must be greater than or equal to min");
	}

	return min + Math.floor(Math.random() * (max - min + 1));
}

function appendPath(endpoint: string, path: string): URL {
	const url = new URL(endpoint);
	const prefix = url.pathname.replace(/\/$/, "");
	url.pathname = `${prefix}${path}`;
	url.search = "";
	url.hash = "";
	return url;
}

function buildSleepUrl(actorId: string): string {
	const url = appendPath(
		ENDPOINT,
		`/actors/${encodeURIComponent(actorId)}/sleep`,
	);
	url.searchParams.set("namespace", NAMESPACE);
	return url.toString();
}

function buildWebSocketUrl(actorId: string): string {
	const tokenSegment = TOKEN ? `@${encodeURIComponent(TOKEN)}` : "";
	const url = appendPath(
		ENDPOINT,
		`/gateway/${encodeURIComponent(actorId)}${tokenSegment}/websocket`,
	);
	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
	return url.toString();
}

function formatError(error: unknown): string {
	if (error instanceof Error) return `${error.name}: ${error.message}`;
	return String(error);
}

function appendServerLog(logs: string[], chunk: Buffer) {
	const text = chunk.toString();
	logs.push(text);
	while (logs.length > 200) logs.shift();
	if (SERVER_LOGS) process.stdout.write(`[server] ${text}`);
}

function serverLogTail(server: LocalKitchenSinkServer): string {
	return server.logs.join("").slice(-20_000);
}

function resolveEngineBinary(): string {
	if (process.env.RIVET_ENGINE_BINARY) return process.env.RIVET_ENGINE_BINARY;

	const candidate = join(REPO_ROOT, "target/debug/rivet-engine");
	if (existsSync(candidate)) return candidate;

	throw new Error(
		`No local rivet-engine binary found. Build one with cargo build -p rivet-engine or set RIVET_ENGINE_BINARY.`,
	);
}

async function waitForLocalServerReady(
	server: LocalKitchenSinkServer,
	serverlessUrl: string,
) {
	const deadline = Date.now() + SERVER_READY_TIMEOUT_MS;
	let lastError: unknown;

	while (Date.now() < deadline) {
		if (server.child.exitCode !== null) {
			throw new Error(
				`kitchen sink server exited before metadata was ready:\n${serverLogTail(server)}`,
			);
		}

		try {
			const response = await fetch(`${serverlessUrl.replace(/\/$/, "")}/metadata`);
			if (response.ok) return;
			lastError = new Error(`metadata returned ${response.status}`);
		} catch (error) {
			lastError = error;
		}

		await sleep(250);
	}

	throw new Error(
		`timed out waiting for kitchen sink server metadata after ${SERVER_READY_TIMEOUT_MS}ms: ${formatError(lastError)}\n${serverLogTail(server)}`,
	);
}

async function startLocalKitchenSinkServer() {
	if (!SERVERLESS_URL) {
		throw new Error("SERVERLESS_URL is required to start the local server");
	}

	const dbRoot = mkdtempSync(join(tmpdir(), "mock-agentic-loop-engine-"));
	const enginePort = portFromUrl(ENDPOINT);
	const serverPort = portFromUrl(SERVERLESS_URL);
	await assertPortAvailable(enginePort, "engine endpoint");
	await assertPortAvailable(serverPort, "kitchen sink server");
	const runId = crypto.randomUUID();
	const logs: string[] = [];
	const child = spawn(
		process.execPath,
		[
			"--import",
			"@rivetkit/sql-loader",
			"--import",
			"tsx",
			"src/server.ts",
		],
		{
			cwd: KITCHEN_SINK_DIR,
			detached: true,
			env: {
				...process.env,
				RIVET_RUN_ENGINE: "1",
				RIVET_ENGINE_BINARY: resolveEngineBinary(),
				RIVETKIT_RUNTIME: process.env.RIVETKIT_RUNTIME ?? "native",
				RIVETKIT_STORAGE_PATH:
					process.env.RIVETKIT_STORAGE_PATH ?? dbRoot,
				RIVET_SERVERLESS_URL: SERVERLESS_URL,
				RIVET__FILE_SYSTEM__PATH:
					process.env.RIVET__FILE_SYSTEM__PATH ?? join(dbRoot, "db"),
				MOCK_AGENTIC_ENGINE_RUN_ID: runId,
				_RIVET_METRICS_TOKEN:
					process.env._RIVET_METRICS_TOKEN ??
					process.env.MOCK_AGENTIC_METRICS_TOKEN ??
					"dev-metrics",
			},
			stdio: ["ignore", "pipe", "pipe"],
		},
	);
	const server: LocalKitchenSinkServer = {
		child,
		dbRoot,
		enginePort,
		runId,
		serverPort,
		logs,
	};
	child.stdout.on("data", (chunk) => appendServerLog(logs, chunk));
	child.stderr.on("data", (chunk) => appendServerLog(logs, chunk));

	try {
		console.log(`[server] starting url=${SERVERLESS_URL}`);
		await waitForLocalServerReady(server, SERVERLESS_URL);
		console.log(`[server] ready url=${SERVERLESS_URL}`);
		return server;
	} catch (error) {
		await stopLocalKitchenSinkServer(server);
		throw error;
	}
}

async function stopLocalKitchenSinkServer(
	server: LocalKitchenSinkServer | undefined,
) {
	if (!server) return;

	const { child, dbRoot } = server;
	if (child.exitCode === null) {
		if (child.pid !== undefined) {
			try {
				process.kill(-child.pid, "SIGTERM");
			} catch {
				child.kill("SIGTERM");
			}
		} else {
			child.kill("SIGTERM");
		}
		await Promise.race([
			new Promise<void>((resolve) => child.once("exit", () => resolve())),
			sleep(5_000),
		]);
		if (child.exitCode === null) {
			if (child.pid !== undefined) {
				try {
					process.kill(-child.pid, "SIGKILL");
				} catch {
					child.kill("SIGKILL");
				}
			} else {
				child.kill("SIGKILL");
			}
		}
	}
	await stopProcessesWithEnvValue("MOCK_AGENTIC_ENGINE_RUN_ID", server.runId);
	rmSync(dbRoot, { recursive: true, force: true });
	await stopListeners(server.enginePort, "engine");
	await stopListeners(server.serverPort, "kitchen sink server");
	console.log("[server] stopped");
}

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;

	await new Promise<void>((resolve, reject) => {
		const timeout = setTimeout(() => {
			cleanup();
			reject(new Error(`websocket open timed out after ${OPEN_TIMEOUT_MS}ms`));
		}, OPEN_TIMEOUT_MS);
		const cleanup = () => {
			clearTimeout(timeout);
			ws.removeEventListener("open", onOpen);
			ws.removeEventListener("error", onError);
			ws.removeEventListener("close", onClose);
		};
		const onOpen = () => {
			cleanup();
			resolve();
		};
		const onError = () => {
			cleanup();
			reject(new Error("websocket error"));
		};
		const onClose = (event: CloseEvent) => {
			cleanup();
			reject(
				new Error(
					`websocket closed before open code=${event.code} reason=${event.reason}`,
				),
			);
		};
		ws.addEventListener("open", onOpen, { once: true });
		ws.addEventListener(
			"error",
			onError,
			{
				once: true,
			},
		);
		ws.addEventListener(
			"close",
			onClose,
			{ once: true },
		);
	});
}

class RawSession {
	#ws: WebSocket | undefined;
	#waiters: Waiter[] = [];
	#backlog: ServerMessage[] = [];
	#closeWaiters: Array<{
		resolve: (event: CloseObservation) => void;
		reject: (error: Error) => void;
		timeout: NodeJS.Timeout;
	}> = [];
	readonly closeEvents: CloseObservation[] = [];

	constructor(
		readonly url: string,
		readonly label: string,
	) {}

	get open() {
		return this.#ws?.readyState === WebSocket.OPEN;
	}

	async connect() {
		if (this.open) return 0;

		const startedAt = Date.now();
		const ws = new WebSocket(this.url, ["rivet", "rivet_encoding.json"]);
		this.#ws = ws;
		ws.addEventListener("message", (event) => this.#onMessage(event));
		ws.addEventListener(
			"close",
			(event) => {
				if (this.#ws === ws) this.#ws = undefined;
				const observation = {
					code: event.code,
					reason: event.reason,
					timestamp: Date.now(),
					phase: "main",
				};
				this.closeEvents.push(observation);
				this.#resolveCloseWaiters(observation);
				this.#rejectWaiters(
					new Error(
						`websocket closed code=${event.code} reason=${event.reason}`,
					),
				);
			},
			{ once: true },
		);
		await waitForOpen(ws);
		const openMs = Date.now() - startedAt;
		console.log(`[connect] ${this.label} openMs=${openMs}`);
		return openMs;
	}

	send(payload: unknown) {
		if (!this.open || !this.#ws) {
			throw new Error("websocket is not open");
		}
		this.#ws.send(JSON.stringify(payload));
	}

	waitFor(
		accept: (message: ServerMessage) => boolean,
		timeoutMs: number,
	): Promise<ServerMessage> {
		const backlogIndex = this.#backlog.findIndex(accept);
		if (backlogIndex !== -1) {
			const [message] = this.#backlog.splice(backlogIndex, 1);
			return Promise.resolve(message);
		}

		return new Promise((resolve, reject) => {
			const waiter: Waiter = {
				accept,
				resolve,
				reject,
				timeout: setTimeout(() => {
					this.#waiters = this.#waiters.filter(
						(item) => item !== waiter,
					);
					reject(new Error(`timed out after ${timeoutMs}ms`));
				}, timeoutMs),
			};
			this.#waiters.push(waiter);
		});
	}

	waitForClose(timeoutMs: number): Promise<CloseObservation> {
		if (!this.open && this.closeEvents.length > 0) {
			return Promise.resolve(this.closeEvents[this.closeEvents.length - 1]);
		}

		return new Promise((resolve, reject) => {
			const waiter = {
				resolve,
				reject,
				timeout: setTimeout(() => {
					this.#closeWaiters = this.#closeWaiters.filter(
						(item) => item !== waiter,
					);
					reject(new Error(`timed out waiting for close after ${timeoutMs}ms`));
				}, timeoutMs),
			};
			this.#closeWaiters.push(waiter);
		});
	}

	close() {
		this.#ws?.close(1000, "mock agentic loop complete");
		this.#ws = undefined;
		this.#rejectWaiters(new Error("websocket closed by client"));
	}

	#onMessage(event: MessageEvent) {
		if (typeof event.data !== "string") {
			throw new Error("received non-string websocket message");
		}

		const message = JSON.parse(event.data) as ServerMessage;
		if (message.type === "error") {
			this.#rejectWaiters(new Error(message.message));
			return;
		}

		for (const waiter of this.#waiters) {
			if (!waiter.accept(message)) continue;
			clearTimeout(waiter.timeout);
			this.#waiters = this.#waiters.filter((item) => item !== waiter);
			waiter.resolve(message);
			return;
		}
		this.#backlog.push(message);
	}

	#rejectWaiters(error: Error) {
		const waiters = this.#waiters;
		this.#waiters = [];
		for (const waiter of waiters) {
			clearTimeout(waiter.timeout);
			waiter.reject(error);
		}
	}

	#resolveCloseWaiters(event: CloseObservation) {
		const waiters = this.#closeWaiters;
		this.#closeWaiters = [];
		for (const waiter of waiters) {
			clearTimeout(waiter.timeout);
			waiter.resolve(event);
		}
	}
}

async function postSleep(actorId: string, stopAt: number, stats: SleepStats) {
	const sleepUrl = buildSleepUrl(actorId);
	let nextSleepAt = Date.now() + SLEEP_INTERVAL_MS;

	while (nextSleepAt < stopAt) {
		await sleep(Math.max(0, nextSleepAt - Date.now()));
		if (Date.now() >= stopAt) break;

		stats.posts += 1;
		stats.postTimes.push(Date.now());
		try {
			console.log(`[sleep] post=${stats.posts} url=${sleepUrl}`);
			const response = await fetch(sleepUrl, {
				method: "POST",
				headers: {
					Authorization: TOKEN ? `Bearer ${TOKEN}` : "",
					"content-type": "application/json",
				},
				body: "{}",
			});
			const body = await response.text();
			console.log(
				`[sleep] post=${stats.posts} status=${response.status} body=${body}`,
			);
			if (!response.ok) stats.errors += 1;
		} catch (error) {
			stats.errors += 1;
			console.error(
				`[sleep-error] post=${stats.posts} ${formatError(error)}`,
			);
		}

		nextSleepAt += SLEEP_INTERVAL_MS;
	}

	return stats;
}

async function triggerServerlessConfiguration() {
	if (!SERVERLESS_URL) return;

	const url = `${SERVERLESS_URL.replace(/\/$/, "")}/metadata`;
	console.log(`[configure] hitting ${url}`);
	const response = await fetch(url);
	console.log(`[configure] status=${response.status}`);
	if (!response.ok) {
		throw new Error(`serverless metadata returned ${response.status}`);
	}
}

async function requestHistory(session: RawSession) {
	session.send({ type: "history" });
	const history = await session.waitFor(
		(message) => message.type === "history",
		10_000,
	);
	if (history.type !== "history") {
		throw new Error("expected history response");
	}
	console.log(`[history] totalRows=${history.totalRows}`);
	return history;
}

function validateHistory(
	history: Extract<ServerMessage, { type: "history" }>,
	expectedRequests: RequestExpectation[],
) {
	const expectedByRequest = new Map(
		expectedRequests.map((request) => [request.requestId, request.seconds]),
	);
	const rowsByRequest = new Map<string, HistoryEntry[]>();

	for (const entry of history.entries) {
		const rows = rowsByRequest.get(entry.request_id) ?? [];
		rows.push(entry);
		rowsByRequest.set(entry.request_id, rows);
	}

	const expectedTotalRows = expectedRequests.reduce(
		(total, request) => total + request.seconds,
		0,
	);
	if (history.totalRows !== expectedTotalRows) {
		throw new Error(
			`history totalRows expected ${expectedTotalRows}, got ${history.totalRows}`,
		);
	}
	if (history.entries.length !== expectedTotalRows) {
		throw new Error(
			`history entries expected ${expectedTotalRows}, got ${history.entries.length}`,
		);
	}

	const unexpected = [...rowsByRequest.keys()].filter(
		(requestId) => !expectedByRequest.has(requestId),
	);
	if (unexpected.length > 0) {
		throw new Error(`history had unexpected request ids: ${unexpected.join(",")}`);
	}

	for (const request of expectedRequests) {
		const indexes = (rowsByRequest.get(request.requestId) ?? [])
			.map((entry) => entry.idx)
			.sort((a, b) => a - b);
		const contiguous =
			indexes.length === request.seconds &&
			indexes.every((idx, offset) => idx === offset + 1);
		if (!contiguous) {
			throw new Error(
				`history request ${request.requestId} expected 1..${request.seconds}, got ${JSON.stringify(indexes)}`,
			);
		}
	}
}

async function verifyAll(
	verifier: ActionVerifier,
	expectedRequests: RequestExpectation[],
) {
	const verified = await verifier.verifyAll(expectedRequests);
	if (!verified.ok) {
		throw new Error(
			`aggregate verification failed: ${JSON.stringify(verified)}`,
		);
	}
	console.log(
		`[verified-all] requests=${verified.expectedRequests} rows=${verified.totalRows}`,
	);
}

async function verifyRequest(
	verifier: ActionVerifier,
	requestId: string,
	seconds: number,
	startedAt: number,
) {
	const verified = await verifier.verify(requestId, seconds);
	const contiguous =
		verified.count === seconds &&
		verified.indexes.every((idx, offset) => idx === offset + 1);
	if (!contiguous) {
		throw new Error(
			`request ${requestId} failed explicit verification: ${JSON.stringify(verified)}`,
		);
	}

	console.log(
		`[verified] requestId=${requestId} rows=${verified.count} elapsedMs=${Date.now() - startedAt}`,
	);
}

function expectedProbeClose(observation: CloseObservation) {
	return (
		observation.code === EXPECTED_PROBE_CLOSE_CODE &&
		observation.reason.startsWith(EXPECTED_PROBE_CLOSE_REASON_PREFIX)
	);
}

function isTransientConnectError(error: unknown) {
	const message = formatError(error);
	return (
		message.includes("actor.stopping") ||
		message.includes("guard.actor_ready_timeout") ||
		message.includes("guard.service_unavailable") ||
		message.includes("guard.websocket_service_unavailable")
	);
}

async function connectAndValidateHistory(
	session: RawSession,
	expectedRequests: RequestExpectation[],
	maxElapsedMs: number,
) {
	const startedAt = Date.now();
	let attempts = 0;

	while (true) {
		attempts += 1;
		try {
			await session.connect();
			validateHistory(await requestHistory(session), expectedRequests);
			const elapsedMs = Date.now() - startedAt;
			if (attempts > 1) {
				console.log(`[connect-ready] attempts=${attempts} elapsedMs=${elapsedMs}`);
			}
			return elapsedMs;
		} catch (error) {
			const elapsedMs = Date.now() - startedAt;
			if (!isTransientConnectError(error) || elapsedMs >= maxElapsedMs) {
				throw error;
			}
			console.log(
				`[connect-retry] attempts=${attempts} elapsedMs=${elapsedMs} error=${formatError(error)}`,
			);
			session.close();
			await sleep(Math.min(RECONNECT_DELAY_MS, maxElapsedMs - elapsedMs));
		}
	}
}

async function runProbeAttempt(webSocketUrl: string, stats: ProbeStats) {
	stats.attempts += 1;
	const probeId = crypto.randomUUID();
	const ws = new WebSocket(webSocketUrl, ["rivet", "rivet_encoding.json"]);
	let closePhase = "open";

	try {
		const closePromise = new Promise<CloseObservation>((resolve, reject) => {
			const onClose = (event: CloseEvent) => {
				cleanup();
				resolve({
					code: event.code,
					reason: event.reason,
					timestamp: Date.now(),
					phase: closePhase,
				});
			};
			const onError = () => {
				cleanup();
				reject(new Error(`probe ${closePhase} websocket error`));
			};
			const cleanup = () => {
				ws.removeEventListener("close", onClose);
				ws.removeEventListener("error", onError);
			};
			ws.addEventListener("close", onClose, { once: true });
			ws.addEventListener("error", onError, { once: true });
		});
		const timeout = (phase: string) =>
			new Promise<never>((_resolve, reject) => {
				setTimeout(
					() =>
						reject(
							new Error(`probe ${phase} timed out after ${PROBE_TIMEOUT_MS}ms`),
						),
					PROBE_TIMEOUT_MS,
				);
			});

		const openResult = await Promise.race([
			waitForOpen(ws).then(() => "open" as const),
			closePromise,
			timeout("open"),
		]);
		if (openResult !== "open") {
			throw Object.assign(new Error("probe closed before open"), {
				close: openResult,
			});
		}

		const pong = new Promise<void>((resolve, reject) => {
			const timeout = setTimeout(() => {
				cleanup();
				reject(new Error("probe timed out waiting for pong"));
			}, PROBE_TIMEOUT_MS);
			const cleanup = () => {
				clearTimeout(timeout);
				ws.removeEventListener("message", onMessage);
				ws.removeEventListener("close", onClose);
				ws.removeEventListener("error", onError);
			};
			const onMessage = (event: MessageEvent) => {
				if (typeof event.data !== "string") return;
				const message = JSON.parse(event.data) as ServerMessage;
				if (message.type !== "pong" || message.probeId !== probeId) {
					return;
				}
				cleanup();
				resolve();
			};
			const onClose = (event: CloseEvent) => {
				cleanup();
				const close: CloseObservation = {
					code: event.code,
					reason: event.reason,
					timestamp: Date.now(),
					phase: "pong",
				};
				reject(Object.assign(new Error("probe closed before pong"), { close }));
			};
			const onError = () => {
				cleanup();
				reject(new Error("probe websocket error before pong"));
			};
			ws.addEventListener("message", onMessage);
			ws.addEventListener("close", onClose, { once: true });
			ws.addEventListener("error", onError, { once: true });
		});

		closePhase = "pong";
		ws.send(JSON.stringify({ type: "ping", probeId }));
		const pongResult = await Promise.race([
			pong.then(() => "pong" as const),
			closePromise,
			timeout("pong"),
		]);
		if (pongResult !== "pong") {
			throw Object.assign(new Error("probe closed before pong"), {
				close: pongResult,
			});
		}
		stats.successes += 1;
		ws.close(1000, "probe complete");
	} catch (error) {
		const close = (error as { close?: CloseObservation }).close;
		if (close) {
			if (expectedProbeClose(close)) {
				stats.expectedCloses += 1;
				stats.expectedCloseSamples.push(close);
				console.log(
					`[probe-close] code=${close.code} reason=${close.reason} phase=${close.phase}`,
				);
				return;
			}

			stats.unexpectedCloses.push(close);
			return;
		}

		const message = formatError(error);
		if (message.includes("timed out")) {
			stats.timeouts += 1;
			console.error(`[probe-timeout] ${message}`);
		} else {
			stats.errors.push(message);
			console.error(`[probe-error] ${message}`);
		}
	} finally {
		if (
			ws.readyState === WebSocket.OPEN ||
			ws.readyState === WebSocket.CONNECTING
		) {
			ws.close(1000, "probe cleanup");
		}
	}
}

async function runProbeLoop(webSocketUrl: string, stopAt: number) {
	const stats: ProbeStats = {
		attempts: 0,
		successes: 0,
		expectedCloses: 0,
		unexpectedCloses: [],
		timeouts: 0,
		errors: [],
		expectedCloseSamples: [],
	};
	let nextProbeAt = Date.now();
	const pending = new Set<Promise<void>>();

	while (Date.now() < stopAt) {
		await sleep(Math.max(0, nextProbeAt - Date.now()));
		if (Date.now() >= stopAt) break;
		const attempt = runProbeAttempt(webSocketUrl, stats).finally(() => {
			pending.delete(attempt);
		});
		pending.add(attempt);
		nextProbeAt += PROBE_INTERVAL_MS;
	}

	await Promise.all(pending);
	return stats;
}

function validateBypassSleepStatus(
	source: string,
	value: {
		sleepStarted?: unknown;
		sleepStartedAt?: unknown;
	},
) {
	if (typeof value.sleepStarted !== "boolean") {
		throw new Error(`${source} missing boolean sleepStarted`);
	}
	if (value.sleepStarted) {
		if (typeof value.sleepStartedAt !== "number") {
			throw new Error(`${source} missing numeric sleepStartedAt`);
		}
	} else if (value.sleepStartedAt !== null) {
		throw new Error(`${source} expected null sleepStartedAt before sleep`);
	}

	return {
		sleepStarted: value.sleepStarted,
		sleepStartedAt: value.sleepStartedAt,
	};
}

async function runBypassAttempt(
	handle: BypassHandle,
	stats: BypassStats,
	phase: BypassPhase,
) {
	stats.attempts += 1;
	if (phase === "beforeSleep") {
		stats.beforeSleepAttempts += 1;
	} else {
		stats.afterSleepAttempts += 1;
	}
	const probeId = crypto.randomUUID();

	try {
		const controller = new AbortController();
		const abortTimeout = setTimeout(
			() => controller.abort(),
			BYPASS_TIMEOUT_MS,
		);
		try {
			const response = await withTimeout(
				handle.fetch(`/bypass?probe=${encodeURIComponent(probeId)}`, {
					method: "GET",
					signal: controller.signal,
					gateway: {
						skipReadyWait: true,
					},
				}),
				"bypass http",
				BYPASS_TIMEOUT_MS,
			);
			if (!response.ok) {
				throw new Error(
					`bypass http returned ${response.status}: ${await response.text()}`,
				);
			}
			const body = (await response.json()) as {
				type?: string;
				transport?: string;
				sleepStarted?: unknown;
				sleepStartedAt?: unknown;
			};
			if (body.type !== "bypass" || body.transport !== "http") {
				throw new Error(`unexpected bypass http body ${JSON.stringify(body)}`);
			}
			const sleepStatus = validateBypassSleepStatus("bypass http", body);
			stats.httpSuccesses += 1;
			if (phase === "beforeSleep") {
				stats.beforeSleepHttpSuccesses += 1;
				if (sleepStatus.sleepStarted) {
					stats.beforeSleepHttpUnexpectedSleepStarted += 1;
				}
			} else {
				stats.afterSleepHttpSuccesses += 1;
				if (sleepStatus.sleepStarted) {
					stats.afterSleepHttpSleepStarted += 1;
				}
			}
		} finally {
			clearTimeout(abortTimeout);
		}

		const ws = await withTimeout(
			handle.webSocket("/bypass", undefined, {
				gateway: {
					skipReadyWait: true,
				},
			}),
			"bypass websocket create",
			BYPASS_TIMEOUT_MS,
		);
		try {
			await withTimeout(
				waitForOpen(ws),
				"bypass websocket open",
				BYPASS_TIMEOUT_MS,
			);
			let webSocketSleepStarted = false;
			const pong = new Promise<void>((resolve, reject) => {
				const timeoutHandle = setTimeout(() => {
					cleanup();
					reject(new Error("bypass websocket timed out waiting for pong"));
				}, BYPASS_TIMEOUT_MS);
				const cleanup = () => {
					clearTimeout(timeoutHandle);
					ws.removeEventListener("message", onMessage);
					ws.removeEventListener("close", onClose);
					ws.removeEventListener("error", onError);
				};
				const onMessage = (event: MessageEvent) => {
					if (typeof event.data !== "string") return;
					const message = JSON.parse(event.data) as ServerMessage;
					if (message.type !== "pong" || message.probeId !== probeId) {
						return;
					}
					webSocketSleepStarted = validateBypassSleepStatus(
						"bypass websocket",
						message,
					).sleepStarted;
					cleanup();
					resolve();
				};
				const onClose = (event: CloseEvent) => {
					cleanup();
					reject(
						new Error(
							`bypass websocket closed code=${event.code} reason=${event.reason}`,
						),
					);
				};
				const onError = () => {
					cleanup();
					reject(new Error("bypass websocket error"));
				};
				ws.addEventListener("message", onMessage);
				ws.addEventListener("close", onClose, { once: true });
				ws.addEventListener("error", onError, { once: true });
			});
			ws.send(JSON.stringify({ type: "ping", probeId }));
			await pong;
			stats.webSocketSuccesses += 1;
			if (phase === "beforeSleep") {
				stats.beforeSleepWebSocketSuccesses += 1;
				if (webSocketSleepStarted) {
					stats.beforeSleepWebSocketUnexpectedSleepStarted += 1;
				}
			} else {
				stats.afterSleepWebSocketSuccesses += 1;
				if (webSocketSleepStarted) {
					stats.afterSleepWebSocketSleepStarted += 1;
				}
			}
		} finally {
			if (
				ws.readyState === WebSocket.OPEN ||
				ws.readyState === WebSocket.CONNECTING
			) {
				ws.close(1000, "bypass probe complete");
			}
		}
	} catch (error) {
		const message = formatError(error);
		const observation = { phase, message };
		if (message.includes("timed out")) {
			stats.timeouts.push(observation);
			console.error(`[bypass-timeout] phase=${phase} ${message}`);
		} else {
			stats.errors.push(observation);
			console.error(`[bypass-error] phase=${phase} ${message}`);
		}
	}
}

async function runBypassLoop(
	handle: BypassHandle,
	stopAt: number,
	getPhase: () => BypassPhase,
) {
	const stats: BypassStats = {
		attempts: 0,
		beforeSleepAttempts: 0,
		afterSleepAttempts: 0,
		httpSuccesses: 0,
		beforeSleepHttpSuccesses: 0,
		afterSleepHttpSuccesses: 0,
		beforeSleepHttpUnexpectedSleepStarted: 0,
		afterSleepHttpSleepStarted: 0,
		webSocketSuccesses: 0,
		beforeSleepWebSocketSuccesses: 0,
		afterSleepWebSocketSuccesses: 0,
		beforeSleepWebSocketUnexpectedSleepStarted: 0,
		afterSleepWebSocketSleepStarted: 0,
		timeouts: [],
		errors: [],
	};
	let nextProbeAt = Date.now();
	const pending = new Set<Promise<void>>();

	while (Date.now() < stopAt) {
		await sleep(Math.max(0, nextProbeAt - Date.now()));
		if (Date.now() >= stopAt) break;
		const attempt = runBypassAttempt(handle, stats, getPhase()).finally(
			() => {
				pending.delete(attempt);
			},
		);
		pending.add(attempt);
		nextProbeAt += BYPASS_INTERVAL_MS;
	}

	await Promise.all(pending);
	return stats;
}

async function runInference(
	session: RawSession,
	requestId: string,
	seconds: number,
) {
	const startedAt = Date.now();
	const progressTimeoutMs = 1_000 + PROGRESS_MARGIN_MS;
	let expectedIdx = 1;
	let lastProgressAt = startedAt;

	console.log(`[infer] requestId=${requestId} seconds=${seconds}`);
	session.send({ type: "infer", requestId, seconds });

	while (expectedIdx <= seconds) {
		const message = await session.waitFor(
			(candidate) =>
				(candidate.type === "progress" || candidate.type === "done") &&
				candidate.requestId === requestId,
			progressTimeoutMs,
		);

		if (message.type === "done") {
			throw new Error(
				`request ${requestId} finished before progress idx=${expectedIdx}`,
			);
		}
		if (message.type !== "progress") {
			throw new Error(`request ${requestId} received unexpected message`);
		}

		const now = Date.now();
		const gapMs = now - lastProgressAt;
		if (gapMs > progressTimeoutMs) {
			throw new Error(
				`request ${requestId} progress gap ${gapMs}ms exceeded ${progressTimeoutMs}ms`,
			);
		}
		if (message.idx !== expectedIdx) {
			throw new Error(
				`request ${requestId} expected idx=${expectedIdx}, got idx=${message.idx}`,
			);
		}

		console.log(
			`[progress] requestId=${requestId} idx=${message.idx}/${seconds} gapMs=${gapMs}`,
		);
		expectedIdx += 1;
		lastProgressAt = now;
	}

	const done = await session.waitFor(
		(candidate) =>
			candidate.type === "done" && candidate.requestId === requestId,
		progressTimeoutMs,
	);
	if (done.type !== "done") {
		throw new Error("expected done response");
	}
	if (!done.verification.ok) {
		throw new Error(
			`request ${requestId} done verification failed: ${JSON.stringify(done.verification)}`,
		);
	}
	return startedAt;
}

async function runWorkload() {
	if (
		INFERENCE_MIN_SECONDS < 1 ||
		INFERENCE_MAX_SECONDS < INFERENCE_MIN_SECONDS
	) {
		throw new Error("invalid inference second range");
	}
	if (JITTER_MIN_MS < 0 || JITTER_MAX_MS < JITTER_MIN_MS) {
		throw new Error("invalid jitter range");
	}

	await triggerServerlessConfiguration();

	const key = `${KEY_PREFIX}-${new Date().toISOString()}-${crypto.randomUUID()}`;
	const label = `key=${key}`;
	const client = createClient<typeof registry>({
		endpoint: ENDPOINT,
		namespace: NAMESPACE,
		token: TOKEN,
		poolName: POOL_NAME,
	});
	const handle = client.mockAgenticLoop.getOrCreate([key]);
	const verifier = handle as unknown as ActionVerifier;
	const bypassHandle = handle as unknown as BypassHandle;
	const actorId = await handle.resolve();
	const webSocketUrl = buildWebSocketUrl(actorId);
	const stopAt = Date.now() + DURATION_MS;
	let requestCount = 0;
	let sleepPostsObservedByMain = 0;
	let reconnectCount = 0;
	let maxReconnectMs = 0;
	const expectedRequests: RequestExpectation[] = [];
	const sleepStats: SleepStats = {
		posts: 0,
		errors: 0,
		postTimes: [],
	};

	console.log(
		`[start] endpoint=${ENDPOINT} namespace=${NAMESPACE} pool=${POOL_NAME} actorId=${actorId} ${label} durationMs=${DURATION_MS} sleepIntervalMs=${SLEEP_INTERVAL_MS} onSleepDelayMs=${ON_SLEEP_DELAY_MS} sleepCloseTimeoutMs=${SLEEP_CLOSE_TIMEOUT_MS} inferenceSeconds=${INFERENCE_MIN_SECONDS}-${INFERENCE_MAX_SECONDS} jitterMs=${JITTER_MIN_MS}-${JITTER_MAX_MS} probeIntervalMs=${PROBE_INTERVAL_MS} bypassIntervalMs=${BYPASS_INTERVAL_MS}`,
	);

	const session = new RawSession(webSocketUrl, label);
	const sleepResultPromise = postSleep(actorId, stopAt, sleepStats);
	let probeResultPromise: Promise<ProbeStats> | undefined;
	let bypassResultPromise: Promise<BypassStats> | undefined;

	try {
		await connectAndValidateHistory(session, expectedRequests, MAX_RECONNECT_MS);
		probeResultPromise = runProbeLoop(webSocketUrl, stopAt);
		bypassResultPromise = runBypassLoop(
			bypassHandle,
			stopAt,
			() => (sleepStats.posts === 0 ? "beforeSleep" : "afterSleep"),
		);

		while (Date.now() < stopAt) {
			const jitterMs = randomInteger(JITTER_MIN_MS, JITTER_MAX_MS);
			if (jitterMs > 0) {
				console.log(`[jitter] delayMs=${jitterMs}`);
				await sleep(
					Math.min(jitterMs, Math.max(0, stopAt - Date.now())),
				);
			}
			if (Date.now() >= stopAt) break;

			if (!session.open) {
				if (sleepStats.posts > sleepPostsObservedByMain) {
					const close =
						session.closeEvents[session.closeEvents.length - 1];
					if (!close) {
						throw new Error("main websocket closed without a close event");
					}
					console.log(
						`[sleep-close] code=${close.code} reason=${close.reason}`,
					);
					sleepPostsObservedByMain = sleepStats.posts;
				}
				await sleep(RECONNECT_DELAY_MS);
				const reconnectMs = await connectAndValidateHistory(
					session,
					expectedRequests,
					MAX_RECONNECT_MS,
				);
				reconnectCount += 1;
				maxReconnectMs = Math.max(maxReconnectMs, reconnectMs);
				if (reconnectMs > MAX_RECONNECT_MS) {
					throw new Error(
						`reconnect took ${reconnectMs}ms, exceeded ${MAX_RECONNECT_MS}ms`,
					);
				}
			}

			requestCount += 1;
			const seconds = randomInteger(
				INFERENCE_MIN_SECONDS,
				INFERENCE_MAX_SECONDS,
			);
			const requestId = crypto.randomUUID();
			const startedAt = await runInference(session, requestId, seconds);
			expectedRequests.push({ requestId, seconds });

			if (sleepStats.posts > sleepPostsObservedByMain) {
				const close = await session.waitForClose(SLEEP_CLOSE_TIMEOUT_MS);
				console.log(
					`[sleep-close] code=${close.code} reason=${close.reason}`,
				);
				sleepPostsObservedByMain = sleepStats.posts;
				await sleep(RECONNECT_DELAY_MS);
				const reconnectMs = await connectAndValidateHistory(
					session,
					expectedRequests,
					MAX_RECONNECT_MS,
				);
				reconnectCount += 1;
				maxReconnectMs = Math.max(maxReconnectMs, reconnectMs);
				if (reconnectMs > MAX_RECONNECT_MS) {
					throw new Error(
						`reconnect took ${reconnectMs}ms, exceeded ${MAX_RECONNECT_MS}ms`,
					);
				}
			}
			await verifyRequest(verifier, requestId, seconds, startedAt);
			await verifyAll(verifier, expectedRequests);
		}
	} finally {
		session.close();
	}

	const sleepResult = await sleepResultPromise;
	const probeResult =
		probeResultPromise !== undefined
			? await probeResultPromise
			: await runProbeLoop(webSocketUrl, Date.now());
	const bypassResult =
		bypassResultPromise !== undefined
			? await bypassResultPromise
			: await runBypassLoop(bypassHandle, Date.now(), () => "beforeSleep");
	validateHistory(await (async () => {
		const finalSession = new RawSession(webSocketUrl, `${label}:final`);
		const reconnectMs = await connectAndValidateHistory(
			finalSession,
			expectedRequests,
			MAX_RECONNECT_MS,
		);
		reconnectCount += 1;
		maxReconnectMs = Math.max(maxReconnectMs, reconnectMs);
		if (reconnectMs > MAX_RECONNECT_MS) {
			throw new Error(
				`final reconnect took ${reconnectMs}ms, exceeded ${MAX_RECONNECT_MS}ms`,
			);
		}
		finalSession.send({ type: "history" });
		const history = await finalSession.waitFor(
			(message) => message.type === "history",
			10_000,
		);
		if (history.type !== "history") {
			throw new Error("expected history response");
		}
		finalSession.close();
		return history;
	})(), expectedRequests);
	await verifyAll(verifier, expectedRequests);

	console.log(
		`[done] actorId=${actorId} key=${key} requests=${requestCount} sleepPosts=${sleepResult.posts} sleepErrors=${sleepResult.errors} reconnects=${reconnectCount} maxReconnectMs=${maxReconnectMs} probeAttempts=${probeResult.attempts} probeSuccesses=${probeResult.successes} probeExpectedCloses=${probeResult.expectedCloses} bypassAttempts=${bypassResult.attempts} bypassBeforeSleepAttempts=${bypassResult.beforeSleepAttempts} bypassAfterSleepAttempts=${bypassResult.afterSleepAttempts} bypassHttpSuccesses=${bypassResult.httpSuccesses} bypassWebSocketSuccesses=${bypassResult.webSocketSuccesses} bypassBeforeSleepHttpSuccesses=${bypassResult.beforeSleepHttpSuccesses} bypassBeforeSleepWebSocketSuccesses=${bypassResult.beforeSleepWebSocketSuccesses} bypassAfterSleepHttpSuccesses=${bypassResult.afterSleepHttpSuccesses} bypassAfterSleepWebSocketSuccesses=${bypassResult.afterSleepWebSocketSuccesses} bypassAfterSleepHttpSleepStarted=${bypassResult.afterSleepHttpSleepStarted} bypassAfterSleepWebSocketSleepStarted=${bypassResult.afterSleepWebSocketSleepStarted} bypassTimeouts=${bypassResult.timeouts.length} bypassErrors=${bypassResult.errors.length}`,
	);

	if (DURATION_MS >= SLEEP_INTERVAL_MS && sleepResult.posts === 0) {
		throw new Error(
			"duration covered a sleep interval but no sleep posts ran",
		);
	}
	if (sleepResult.errors > 0) {
		throw new Error(`${sleepResult.errors} sleep requests failed`);
	}
	if (sleepResult.posts > 0 && sleepPostsObservedByMain < sleepResult.posts) {
		throw new Error(
			`main websocket observed ${sleepPostsObservedByMain}/${sleepResult.posts} sleep closes`,
		);
	}
	if (sleepResult.posts > 0 && reconnectCount === 0) {
		throw new Error("sleep ran but client never reconnected");
	}
	if (probeResult.unexpectedCloses.length > 0) {
		throw new Error(
			`probe saw unexpected closes: ${JSON.stringify(probeResult.unexpectedCloses)}`,
		);
	}
	if (probeResult.timeouts > 0) {
		throw new Error(`probe had ${probeResult.timeouts} timeouts`);
	}
	if (probeResult.errors.length > 0) {
		throw new Error(`probe errors: ${probeResult.errors.join("; ")}`);
	}
	if (sleepResult.posts > 0 && probeResult.expectedCloses === 0) {
		throw new Error(
			`probe never saw expected close code=${EXPECTED_PROBE_CLOSE_CODE} reasonPrefix=${EXPECTED_PROBE_CLOSE_REASON_PREFIX}`,
		);
	}
	if (bypassResult.attempts === 0) {
		throw new Error("bypass loop did not run");
	}
	if (bypassResult.beforeSleepAttempts === 0) {
		throw new Error("bypass loop did not run before sleep");
	}
	if (
		bypassResult.beforeSleepHttpSuccesses !==
			bypassResult.beforeSleepAttempts ||
		bypassResult.beforeSleepWebSocketSuccesses !==
			bypassResult.beforeSleepAttempts
	) {
		throw new Error(
			`bypass loop failed before sleep: ${JSON.stringify(bypassResult)}`,
		);
	}
	if (
		bypassResult.timeouts.some((item) => item.phase === "beforeSleep") ||
		bypassResult.errors.some((item) => item.phase === "beforeSleep")
	) {
		throw new Error(
			`bypass loop had pre-sleep failures: ${JSON.stringify(bypassResult)}`,
		);
	}
	if (
		bypassResult.beforeSleepHttpUnexpectedSleepStarted > 0 ||
		bypassResult.beforeSleepWebSocketUnexpectedSleepStarted > 0
	) {
		throw new Error(
			`bypass saw sleepStarted before sleep: ${JSON.stringify(bypassResult)}`,
		);
	}
	if (sleepResult.posts > 0 && bypassResult.afterSleepAttempts === 0) {
		throw new Error("bypass loop did not continue after sleep request");
	}
	if (sleepResult.posts > 0 && bypassResult.afterSleepHttpSuccesses === 0) {
		throw new Error("bypass http had no successful after-sleep actor responses");
	}
	if (sleepResult.posts > 0 && bypassResult.afterSleepWebSocketSuccesses === 0) {
		throw new Error("bypass websocket had no successful after-sleep actor responses");
	}
	if (sleepResult.posts > 0 && bypassResult.afterSleepHttpSleepStarted === 0) {
		throw new Error(
			`bypass http never returned actor sleepStarted proof: ${JSON.stringify(bypassResult)}`,
		);
	}
	if (
		sleepResult.posts > 0 &&
		bypassResult.afterSleepWebSocketSleepStarted === 0
	) {
		throw new Error(
			`bypass websocket never returned actor sleepStarted proof: ${JSON.stringify(bypassResult)}`,
		);
	}
}

async function main() {
	const localServer = START_SERVER
		? await startLocalKitchenSinkServer()
		: undefined;
	try {
		await runWorkload();
	} finally {
		await stopLocalKitchenSinkServer(localServer);
	}
}

main().catch((error) => {
	console.error(`[fatal] ${formatError(error)}`);
	process.exitCode = 1;
});
