// Remote onSleep watcher for a kitchen-sink envoy pool.
//
// This does not start the engine or kitchen-sink. Point it at a remote Rivet
// endpoint, then manually roll the envoy pods after the WebSocket is open.
//
// Usage:
//   RIVET_ENDPOINT=https://namespace:token@... \
//   pnpm smoke:on-sleep-remote -- \
//     --pool kitchen-sink \
//     --on-sleep-duration-ms 60000 \
//     --open-delay-ms 0 \
//     --reconnect-timeout-ms 5000

import { createClient } from "rivetkit/client";

installTimestampedConsole();

const CLI_ARGS = parseCliArgs(process.argv.slice(2));
const RAW_ENDPOINT = requiredStringFromConfig("endpoint", ["RIVET_ENDPOINT"]);
const POOL_NAME = stringFromConfig("pool", ["RIVET_POOL"], "default");
const ENDPOINT = parseEndpoint(RAW_ENDPOINT, POOL_NAME);
const KEY = stringFromConfig(
	"key",
	["SIGTERM_SLEEP_KEY"],
	`remote-sleep-probe-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
);
const LABEL = stringFromConfig("label", ["SIGTERM_SLEEP_LABEL"], KEY);
const ON_SLEEP_DURATION_MS = numberFromConfig(
	"on-sleep-duration-ms",
	["SIGTERM_SLEEP_ON_SLEEP_DURATION_MS"],
	60_000,
);
const ON_SLEEP_TICK_MS = numberFromConfig(
	"on-sleep-tick-ms",
	["SIGTERM_SLEEP_ON_SLEEP_TICK_MS"],
	1_000,
);
const OPEN_TIMEOUT_MS = numberFromConfig("open-timeout-ms", [], 15_000);
const OPEN_DELAY_MS = numberFromConfig("open-delay-ms", [], 0);
const MESSAGE_TIMEOUT_MS = numberFromConfig("message-timeout-ms", [], 5_000);
const RECONNECT_TIMEOUT_MS = numberFromConfig(
	"reconnect-timeout-ms",
	[],
	5_000,
);
const WATCH_TIMEOUT_MS = numberFromConfig(
	"watch-timeout-ms",
	[],
	Math.max(5 * 60_000, ON_SLEEP_DURATION_MS + 2 * 60_000),
);
const CLOSE_CODE = 1000;
const CLOSE_REASON = "actor stopped";

type JsonRecord = Record<string, unknown>;

interface CloseInfo {
	code: number;
	reason: string;
	wasClean: boolean;
	at: number;
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

function installTimestampedConsole(): void {
	const originalLog = console.log.bind(console);
	const originalError = console.error.bind(console);
	const originalWarn = console.warn.bind(console);
	console.log = (...args: unknown[]) =>
		originalLog(`[${new Date().toISOString()}]`, ...args);
	console.error = (...args: unknown[]) =>
		originalError(`[${new Date().toISOString()}]`, ...args);
	console.warn = (...args: unknown[]) =>
		originalWarn(`[${new Date().toISOString()}]`, ...args);
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

function requiredStringFromConfig(argName: string, envNames: string[]): string {
	const arg = CLI_ARGS.get(argName);
	if (arg !== undefined && arg !== "") return arg;
	for (const envName of envNames) {
		const raw = process.env[envName];
		if (raw !== undefined && raw !== "") return raw;
	}
	throw new Error(
		`missing required --${argName}. Set ${envNames.join(" or ")} or pass --${argName}.`,
	);
}

function numberFromConfig(
	argName: string,
	envNames: string[],
	fallback: number,
): number {
	const raw =
		CLI_ARGS.get(argName) ??
		envNames.map((envName) => process.env[envName]).find((value) => value);
	if (raw === undefined || raw === "") return fallback;
	const parsed = Number(raw);
	if (!Number.isFinite(parsed) || parsed < 0) {
		throw new Error(`--${argName} must be a finite non-negative number`);
	}
	return parsed;
}

function formatError(error: unknown): string {
	if (error instanceof Error) return `${error.name}: ${error.message}`;
	return String(error);
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function parseEndpoint(raw: string, poolName: string) {
	const url = new URL(raw);
	const namespace = decodeURIComponent(url.username);
	const token = url.password ? decodeURIComponent(url.password) : undefined;
	if (!namespace) {
		throw new Error(
			"RIVET_ENDPOINT must include namespace auth, e.g. https://namespace:token@host",
		);
	}
	url.username = "";
	url.password = "";
	const endpoint = url.toString().replace(/\/$/, "");
	const wsProtocol = url.protocol === "https:" ? "wss:" : "ws:";
	const wsOrigin = `${wsProtocol}//${url.host}`;
	return { endpoint, namespace, token, poolName, wsOrigin };
}

function buildRawWebSocketUrl(): string {
	const params = new URLSearchParams();
	params.set("rvt-namespace", ENDPOINT.namespace);
	params.set("rvt-method", "getOrCreate");
	params.set("rvt-runner", ENDPOINT.poolName);
	params.set("rvt-key", KEY);
	params.set("rvt-crash-policy", "sleep");
	params.set("rvt-skip-ready-wait", "true");
	if (ENDPOINT.token) {
		params.set("rvt-token", ENDPOINT.token);
	}
	return `${ENDPOINT.wsOrigin}/gateway/sigtermSleepProbe/websocket?${params}`;
}

function client() {
	return createClient<any>({
		endpoint: RAW_ENDPOINT,
		poolName: ENDPOINT.poolName,
	});
}

async function waitForOpen(ws: WebSocket, timeoutMs: number): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;
	await new Promise<void>((resolve, reject) => {
		const timeout = setTimeout(
			() =>
				reject(
					new Error(`websocket open timeout after ${timeoutMs}ms`),
				),
			timeoutMs,
		);
		const cleanup = () => clearTimeout(timeout);
		ws.addEventListener(
			"open",
			() => {
				cleanup();
				resolve();
			},
			{ once: true },
		);
		ws.addEventListener(
			"error",
			() => {
				cleanup();
				reject(new Error("websocket error before open"));
			},
			{ once: true },
		);
		ws.addEventListener(
			"close",
			(event) => {
				cleanup();
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
	predicate: (message: JsonRecord) => boolean,
	timeoutMs: number,
	label: string,
): Promise<JsonRecord> {
	return await new Promise((resolve, reject) => {
		const timeout = setTimeout(
			() =>
				reject(
					new Error(`${label} message timeout after ${timeoutMs}ms`),
				),
			timeoutMs,
		);
		const cleanup = () => {
			clearTimeout(timeout);
			ws.removeEventListener("message", onMessage);
			ws.removeEventListener("close", onClose);
		};
		const onMessage = (event: MessageEvent) => {
			const data =
				typeof event.data === "string"
					? event.data
					: String(event.data);
			console.log(`[ws:message] ${data}`);
			let parsed: JsonRecord;
			try {
				parsed = JSON.parse(data) as JsonRecord;
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
		ws.addEventListener("message", onMessage);
		ws.addEventListener("close", onClose, { once: true });
	});
}

function waitForClose(
	ws: WebSocket,
	timeoutMs: number,
	state: {
		sawOnSleepStarted: boolean;
		sawOnSleepFinished: boolean;
		onSleepTickCount: number;
	},
): Promise<CloseInfo> {
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(
			() =>
				reject(
					new Error(`websocket close timeout after ${timeoutMs}ms`),
				),
			timeoutMs,
		);
		ws.addEventListener("message", (event) => {
			const data =
				typeof event.data === "string"
					? event.data
					: String(event.data);
			console.log(`[ws:message] ${data}`);
			try {
				const parsed = JSON.parse(data) as JsonRecord;
				if (parsed.type === "onSleepStarted") {
					state.sawOnSleepStarted = true;
				}
				if (parsed.type === "onSleepTick") {
					state.onSleepTickCount += 1;
				}
				if (parsed.type === "onSleepFinished") {
					state.sawOnSleepFinished = true;
				}
			} catch {
				// Raw non-JSON messages are still logged above.
			}
		});
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

async function connectAndPingPong(
	label: string,
	openTimeoutMs: number,
	messageTimeoutMs: number,
): Promise<WebSocket> {
	const webSocketUrl = buildRawWebSocketUrl();
	console.log(`[ws] ${label} connecting url=${webSocketUrl}`);
	const ws = new WebSocket(webSocketUrl, ["rivet", "rivet_encoding.json"]);
	ws.addEventListener("error", () => {
		console.error(`[ws:error] ${label}`);
	});
	ws.addEventListener("close", (event) => {
		console.log(
			`[ws:close] ${label} code=${event.code} reason=${event.reason} wasClean=${event.wasClean}`,
		);
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
		console.log(`[ws] ${label} ping pong ok`);
		return ws;
	} catch (error) {
		if (
			ws.readyState === WebSocket.OPEN ||
			ws.readyState === WebSocket.CONNECTING
		) {
			ws.close(1000, `${label} failed`);
		}
		throw error;
	}
}

async function reconnectAndPingPong(): Promise<WebSocket> {
	console.log(`[ws] reconnect immediate timeoutMs=${RECONNECT_TIMEOUT_MS}`);
	return await connectAndPingPong(
		"reconnect",
		Math.min(OPEN_TIMEOUT_MS, RECONNECT_TIMEOUT_MS),
		Math.min(MESSAGE_TIMEOUT_MS, RECONNECT_TIMEOUT_MS),
	);
}

function assertClose(close: CloseInfo, startedAt: number): void {
	const elapsedMs = close.at - startedAt;
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
	if (elapsedMs > ON_SLEEP_DURATION_MS + 120_000) {
		throw new Error(
			`websocket closed too late: ${elapsedMs}ms > ${ON_SLEEP_DURATION_MS + 120_000}ms`,
		);
	}
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
		throw new Error(
			`expected sleepCount >= 1, got ${proof.state.sleepCount}`,
		);
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
		throw new Error("--on-sleep-duration-ms must be positive");
	}
	if (ON_SLEEP_TICK_MS <= 0) {
		throw new Error("--on-sleep-tick-ms must be positive");
	}

	console.log(
		`[config] endpoint=${ENDPOINT.endpoint} namespace=${ENDPOINT.namespace} pool=${ENDPOINT.poolName} key=${KEY} durationMs=${ON_SLEEP_DURATION_MS} tickMs=${ON_SLEEP_TICK_MS} openDelayMs=${OPEN_DELAY_MS} reconnectTimeoutMs=${RECONNECT_TIMEOUT_MS} watchTimeoutMs=${WATCH_TIMEOUT_MS}`,
	);

	const handle = client().sigtermSleepProbe.getOrCreate([KEY]);
	const actorId = await handle.resolve();
	console.log(`[actor] actorId=${actorId}`);

	const prepared = await handle.prepare(
		LABEL,
		ON_SLEEP_DURATION_MS,
		ON_SLEEP_TICK_MS,
	);
	console.log(`[actor] prepared=${JSON.stringify(prepared)}`);

	if (OPEN_DELAY_MS > 0) {
		console.log(`[ws] waiting before open delayMs=${OPEN_DELAY_MS}`);
		await sleep(OPEN_DELAY_MS);
	}
	const ws = await connectAndPingPong(
		"initial",
		OPEN_TIMEOUT_MS,
		MESSAGE_TIMEOUT_MS,
	);
	console.log("[manual] roll/restart the remote kitchen-sink envoy pods now");

	const state = {
		sawOnSleepStarted: false,
		sawOnSleepFinished: false,
		onSleepTickCount: 0,
	};
	const startedAt = Date.now();
	const close = await waitForClose(ws, WATCH_TIMEOUT_MS, state);
	const elapsedMs = close.at - startedAt;
	console.log(
		`[ws:close] code=${close.code} reason=${close.reason} wasClean=${close.wasClean} elapsedMs=${elapsedMs}`,
	);
	console.log(
		`[ws] observed sleep messages started=${state.sawOnSleepStarted} ticks=${state.onSleepTickCount} finished=${state.sawOnSleepFinished}`,
	);

	const reconnect = await reconnectAndPingPong();
	reconnect.close(1000, "remote smoke done");

	const proof = (await handle.getProof()) as Proof;
	assertClose(close, startedAt);
	assertProof(proof);
	console.log(`[proof] ${JSON.stringify(proof.state)}`);
	console.log(
		"[done] PASS observed shutdown close, reconnected, and verified onSleep proof",
	);
}

main().catch((error) => {
	console.error(`[fail] ${formatError(error)}`);
	process.exitCode = 1;
});
