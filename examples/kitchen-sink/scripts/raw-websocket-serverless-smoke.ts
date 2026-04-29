// Raw onWebSocket serverless smoke test.
//
// Usage:
//   RIVET_ENDPOINT=http://127.0.0.1:6420 \
//   RIVET_SERVERLESS_URL=http://127.0.0.1:3000/api/rivet \
//   SMOKE_PARALLELISM=4 \
//   SMOKE_STAGGER_MS=1000 \
//   pnpm --filter kitchen-sink smoke:raw-websocket-serverless
//
// The serverless pool is configured by the kitchen-sink server when
// RIVET_SERVERLESS_URL is set. It uses a 30s request lifespan and a 5s drain
// grace period by default.

import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const SERVERLESS_URL = process.env.RIVET_SERVERLESS_URL;
const NAMESPACE =
	process.env.SMOKE_NAMESPACE ?? process.env.RIVET_NAMESPACE ?? "default";
const TOKEN = process.env.SMOKE_TOKEN ?? process.env.RIVET_TOKEN ?? "dev";
const POOL_NAME = process.env.SMOKE_POOL ?? process.env.RIVET_POOL ?? "default";
const KEY = process.env.SMOKE_KEY ?? `raw-ws-serverless-smoke-${Date.now()}`;
const DURATION_MS = Number(process.env.SMOKE_DURATION_MS ?? "120000");
const PARALLELISM = Number(process.env.SMOKE_PARALLELISM ?? "1");
const SHARED_KEY = process.env.SMOKE_SHARED_KEY === "1";
const LOG_MESSAGES = process.env.SMOKE_LOG_MESSAGES !== "0";
const GAP_WARN_MS = Number(process.env.SMOKE_GAP_WARN_MS ?? "3000");
const STALE_TIMEOUT_MS = Number(
	process.env.SMOKE_STALE_TIMEOUT_MS ?? String(GAP_WARN_MS * 2),
);
const SLEEP_INTERVAL_MS = Number(
	process.env.SMOKE_SLEEP_INTERVAL_MS ?? "15000",
);
const STAGGER_MS = Number(process.env.SMOKE_STAGGER_MS ?? "1000");
const POST_SLEEP = process.env.SMOKE_POST_SLEEP !== "0";
const CONNECT_ERROR_DELAY_MS = Number(
	process.env.SMOKE_CONNECT_ERROR_DELAY_MS ?? "250",
);

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function formatError(error: unknown): string {
	if (error instanceof Error) return `${error.name}: ${error.message}`;
	return String(error);
}

function eventDataToString(data: unknown): string {
	if (typeof data === "string") return data;
	if (data instanceof ArrayBuffer) {
		return `<binary ${data.byteLength} bytes>`;
	}
	if (ArrayBuffer.isView(data)) {
		return `<binary ${data.byteLength} bytes>`;
	}
	return String(data);
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

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;

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

async function triggerServerlessConfiguration() {
	if (!SERVERLESS_URL) return;

	const url = `${SERVERLESS_URL.replace(/\/$/, "")}/metadata`;
	console.log(`[configure] hitting ${url}`);
	const response = await fetch(url);
	console.log(`[configure] status=${response.status}`);
}

async function postSleep(actorId: string, label: string, stopAt: number) {
	if (!POST_SLEEP || SLEEP_INTERVAL_MS <= 0) {
		return { sleepPosts: 0, sleepErrors: 0 };
	}

	let sleepPosts = 0;
	let sleepErrors = 0;
	const sleepUrl = buildSleepUrl(actorId);

	while (Date.now() < stopAt) {
		await sleep(Math.min(SLEEP_INTERVAL_MS, Math.max(0, stopAt - Date.now())));
		if (Date.now() >= stopAt) break;

		try {
			sleepPosts += 1;
			console.log(`[sleep] ${label} post=${sleepPosts} url=${sleepUrl}`);
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
				`[sleep] ${label} post=${sleepPosts} status=${response.status} body=${body}`,
			);
		} catch (error) {
			sleepErrors += 1;
			console.error(
				`[sleep-error] ${label} post=${sleepPosts} ${formatError(error)}`,
			);
		}
	}

	return { sleepPosts, sleepErrors };
}

async function runWorker(workerIndex: number, stopAt: number) {
	const startDelayMs = workerIndex * STAGGER_MS;
	if (startDelayMs > 0) {
		await sleep(startDelayMs);
	}

	const key = SHARED_KEY ? KEY : `${KEY}-${workerIndex}`;
	const label = `worker=${workerIndex} key=${key}`;
	const client = createClient<typeof registry>({
		endpoint: ENDPOINT,
		namespace: NAMESPACE,
		token: TOKEN,
		poolName: POOL_NAME,
	});
	const handle = client.rawWebSocketServerlessSmoke.getOrCreate([key]);
	const actorId = await handle.resolve();
	const webSocketUrl = buildWebSocketUrl(actorId);
	let current: WebSocket | undefined;
	let attempt = 0;
	let messageCount = 0;
	let gapCount = 0;
	let staleReconnects = 0;
	let lastGlobalMessageAt = 0;
	const sleepResultPromise = postSleep(actorId, label, stopAt);

	while (Date.now() < stopAt) {
		attempt += 1;
		const openedAt = Date.now();
		let lastMessageAt = 0;
		console.log(
			`[connect] ${label} actorId=${actorId} attempt=${attempt} url=${webSocketUrl}`,
		);
		if (lastGlobalMessageAt > 0) {
			const reconnectGapMs = Date.now() - lastGlobalMessageAt;
			if (reconnectGapMs > GAP_WARN_MS) {
				gapCount += 1;
				console.error(
					`[gap] ${label} attempt=${attempt} reconnectGapMs=${reconnectGapMs} thresholdMs=${GAP_WARN_MS}`,
				);
			}
		}

		try {
			const ws = new WebSocket(webSocketUrl, ["rivet", "rivet_encoding.json"]);
			current = ws;
			await waitForOpen(ws);
			console.log(`[open] ${label} attempt=${attempt}`);

			await new Promise<void>((resolve) => {
				const timeout = setTimeout(
					() => {
						ws.close(1000, "smoke complete");
					},
					Math.max(0, stopAt - Date.now()),
				);
				const staleWatchdog =
					STALE_TIMEOUT_MS > 0
						? setInterval(() => {
								if (
									ws.readyState === WebSocket.OPEN &&
									lastMessageAt > 0 &&
									Date.now() - lastMessageAt > STALE_TIMEOUT_MS
								) {
									staleReconnects += 1;
									console.error(
										`[stale] ${label} attempt=${attempt} lastMessageAgeMs=${Date.now() - lastMessageAt}`,
									);
									ws.close(4000, "stale smoke connection");
								}
							}, Math.min(1000, STALE_TIMEOUT_MS))
						: undefined;

				ws.addEventListener("message", (event) => {
					const now = Date.now();
					const gapMs = lastMessageAt > 0 ? now - lastMessageAt : 0;
					if (gapMs > GAP_WARN_MS) {
						gapCount += 1;
						console.error(
							`[gap] ${label} attempt=${attempt} gapMs=${gapMs} thresholdMs=${GAP_WARN_MS}`,
						);
					}
					lastMessageAt = now;
					lastGlobalMessageAt = now;
					messageCount += 1;
					if (LOG_MESSAGES) {
						console.log(
							`[message] ${label} count=${messageCount} attempt=${attempt} data=${eventDataToString(event.data)}`,
						);
					}
				});
				ws.addEventListener(
					"close",
					(event) => {
						clearTimeout(timeout);
						if (staleWatchdog) clearInterval(staleWatchdog);
						console.log(
							`[close] ${label} attempt=${attempt} code=${event.code} reason=${event.reason} openMs=${Date.now() - openedAt}`,
						);
						resolve();
					},
					{ once: true },
				);
				ws.addEventListener("error", () => {
					console.error(`[error] ${label} attempt=${attempt}`);
				});
			});
		} catch (error) {
			console.error(
				`[connect-error] ${label} attempt=${attempt} ${formatError(error)}`,
			);
			await sleep(CONNECT_ERROR_DELAY_MS);
		} finally {
			current = undefined;
		}
	}

	current?.close(1000, "smoke complete");
	const sleepResult = await sleepResultPromise;
	console.log(
		`[done] ${label} actorId=${actorId} attempts=${attempt} messages=${messageCount} gaps=${gapCount} staleReconnects=${staleReconnects} sleepPosts=${sleepResult.sleepPosts} sleepErrors=${sleepResult.sleepErrors}`,
	);
	return {
		workerIndex,
		attempts: attempt,
		messages: messageCount,
		gaps: gapCount,
		staleReconnects,
		sleepPosts: sleepResult.sleepPosts,
		sleepErrors: sleepResult.sleepErrors,
	};
}

async function main() {
	if (!Number.isInteger(PARALLELISM) || PARALLELISM < 1) {
		throw new Error("SMOKE_PARALLELISM must be a positive integer");
	}

	console.log(
		`[smoke] endpoint=${ENDPOINT} namespace=${NAMESPACE} pool=${POOL_NAME} key=${KEY} durationMs=${DURATION_MS} parallelism=${PARALLELISM} sharedKey=${SHARED_KEY} staggerMs=${STAGGER_MS} gapWarnMs=${GAP_WARN_MS} staleTimeoutMs=${STALE_TIMEOUT_MS} sleepIntervalMs=${SLEEP_INTERVAL_MS} postSleep=${POST_SLEEP}`,
	);
	await triggerServerlessConfiguration();

	const stopAt = Date.now() + DURATION_MS;
	const results = await Promise.all(
		Array.from({ length: PARALLELISM }, (_, i) => runWorker(i, stopAt)),
	);
	const attempts = results.reduce((sum, result) => sum + result.attempts, 0);
	const messages = results.reduce((sum, result) => sum + result.messages, 0);
	const gaps = results.reduce((sum, result) => sum + result.gaps, 0);
	const staleReconnects = results.reduce(
		(sum, result) => sum + result.staleReconnects,
		0,
	);
	const sleepPosts = results.reduce(
		(sum, result) => sum + result.sleepPosts,
		0,
	);
	const sleepErrors = results.reduce(
		(sum, result) => sum + result.sleepErrors,
		0,
	);
	console.log(
		`[summary] workers=${PARALLELISM} attempts=${attempts} messages=${messages} gaps=${gaps} staleReconnects=${staleReconnects} sleepPosts=${sleepPosts} sleepErrors=${sleepErrors}`,
	);
}

main().catch((error) => {
	console.error(`[fatal] ${formatError(error)}`);
	process.exit(1);
});
