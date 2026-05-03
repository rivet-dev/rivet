#!/usr/bin/env -S pnpm exec tsx

import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const SERVERLESS_URL = process.env.RIVET_SERVERLESS_URL;
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
const RECONNECT_DELAY_MS = numberFromEnv(
	"MOCK_AGENTIC_RECONNECT_DELAY_MS",
	500,
);
const SLEEP_INTERVAL_MS = 120_000;

type ServerMessage =
	| { type: "hello"; connectionId: string; timestamp: number }
	| {
			type: "history";
			totalRows: number;
			entries: unknown[];
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

type ActionVerifier = {
	verify: (
		requestId: string,
		expectedSeconds: number,
	) => Promise<ActionVerification>;
};

type Waiter = {
	accept: (message: ServerMessage) => boolean;
	resolve: (message: ServerMessage) => void;
	reject: (error: Error) => void;
	timeout: NodeJS.Timeout;
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

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;

	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener(
			"error",
			() => reject(new Error("websocket error")),
			{
				once: true,
			},
		);
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

class RawSession {
	#ws: WebSocket | undefined;
	#waiters: Waiter[] = [];

	constructor(
		readonly url: string,
		readonly label: string,
	) {}

	get open() {
		return this.#ws?.readyState === WebSocket.OPEN;
	}

	async connect() {
		if (this.open) return;

		const startedAt = Date.now();
		const ws = new WebSocket(this.url, ["rivet", "rivet_encoding.json"]);
		this.#ws = ws;
		ws.addEventListener("message", (event) => this.#onMessage(event));
		ws.addEventListener(
			"close",
			(event) => {
				if (this.#ws === ws) this.#ws = undefined;
				this.#rejectWaiters(
					new Error(
						`websocket closed code=${event.code} reason=${event.reason}`,
					),
				);
			},
			{ once: true },
		);
		await waitForOpen(ws);
		console.log(`[connect] ${this.label} openMs=${Date.now() - startedAt}`);
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
	}

	#rejectWaiters(error: Error) {
		const waiters = this.#waiters;
		this.#waiters = [];
		for (const waiter of waiters) {
			clearTimeout(waiter.timeout);
			waiter.reject(error);
		}
	}
}

async function postSleep(actorId: string, stopAt: number) {
	const sleepUrl = buildSleepUrl(actorId);
	let sleepPosts = 0;
	let sleepErrors = 0;
	let nextSleepAt = Date.now() + SLEEP_INTERVAL_MS;

	while (nextSleepAt < stopAt) {
		await sleep(Math.max(0, nextSleepAt - Date.now()));
		if (Date.now() >= stopAt) break;

		sleepPosts += 1;
		try {
			console.log(`[sleep] post=${sleepPosts} url=${sleepUrl}`);
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
				`[sleep] post=${sleepPosts} status=${response.status} body=${body}`,
			);
			if (!response.ok) sleepErrors += 1;
		} catch (error) {
			sleepErrors += 1;
			console.error(
				`[sleep-error] post=${sleepPosts} ${formatError(error)}`,
			);
		}

		nextSleepAt += SLEEP_INTERVAL_MS;
	}

	return { sleepPosts, sleepErrors };
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
}

async function runInference(
	session: RawSession,
	verifier: ActionVerifier,
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

async function main() {
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
	const actorId = await handle.resolve();
	const webSocketUrl = buildWebSocketUrl(actorId);
	const stopAt = Date.now() + DURATION_MS;
	let requestCount = 0;

	console.log(
		`[start] endpoint=${ENDPOINT} namespace=${NAMESPACE} pool=${POOL_NAME} actorId=${actorId} ${label} durationMs=${DURATION_MS} sleepIntervalMs=${SLEEP_INTERVAL_MS} inferenceSeconds=${INFERENCE_MIN_SECONDS}-${INFERENCE_MAX_SECONDS} jitterMs=${JITTER_MIN_MS}-${JITTER_MAX_MS}`,
	);

	const session = new RawSession(webSocketUrl, label);
	const sleepResultPromise = postSleep(actorId, stopAt);

	try {
		await session.connect();
		await requestHistory(session);

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
				await sleep(RECONNECT_DELAY_MS);
				await session.connect();
			}

			requestCount += 1;
			const seconds = randomInteger(
				INFERENCE_MIN_SECONDS,
				INFERENCE_MAX_SECONDS,
			);
			await runInference(session, handle, crypto.randomUUID(), seconds);
		}
	} finally {
		session.close();
	}

	const sleepResult = await sleepResultPromise;
	console.log(
		`[done] actorId=${actorId} key=${key} requests=${requestCount} sleepPosts=${sleepResult.sleepPosts} sleepErrors=${sleepResult.sleepErrors}`,
	);

	if (DURATION_MS >= SLEEP_INTERVAL_MS && sleepResult.sleepPosts === 0) {
		throw new Error(
			"duration covered a sleep interval but no sleep posts ran",
		);
	}
	if (sleepResult.sleepErrors > 0) {
		throw new Error(`${sleepResult.sleepErrors} sleep requests failed`);
	}
}

main().catch((error) => {
	console.error(`[fatal] ${formatError(error)}`);
	process.exitCode = 1;
});
