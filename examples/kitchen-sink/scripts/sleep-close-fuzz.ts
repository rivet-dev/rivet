// Fuzz test for the force-sleep → non-hibernatable WS close path.
//
// For each parallel worker, repeats:
//   1. getOrCreate a unique sleepCloseFuzz actor
//   2. Open a raw WebSocket and wait for `welcome`
//   3. Wait `WAIT_BEFORE_SLEEP_MS` (a few seconds of normal operation)
//   4. POST {endpoint}/actors/{id}/sleep
//   5. Measure how long until the client close event fires
//   6. Flag a leak if the close does not arrive within `LEAK_THRESHOLD_MS`
//
// Usage:
//   RIVET_ENDPOINT=http://127.0.0.1:6420 \
//   FUZZ_PARALLELISM=20 \
//   FUZZ_DURATION_MS=120000 \
//   pnpm --filter kitchen-sink fuzz:sleep-close

import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const NAMESPACE =
	process.env.FUZZ_NAMESPACE ?? process.env.RIVET_NAMESPACE ?? "default";
const TOKEN = process.env.FUZZ_TOKEN ?? process.env.RIVET_TOKEN ?? "dev";
const PARALLELISM = Number(process.env.FUZZ_PARALLELISM ?? "10");
const DURATION_MS = Number(process.env.FUZZ_DURATION_MS ?? "60000");
const WAIT_BEFORE_SLEEP_MS = Number(process.env.FUZZ_WAIT_BEFORE_SLEEP_MS ?? "2000");
const WAIT_BEFORE_SLEEP_JITTER_MS = Number(
	process.env.FUZZ_WAIT_BEFORE_SLEEP_JITTER_MS ?? "3000",
);
const LEAK_THRESHOLD_MS = Number(process.env.FUZZ_LEAK_THRESHOLD_MS ?? "30000");
const KEY_PREFIX = process.env.FUZZ_KEY_PREFIX ?? `sleep-close-fuzz-${Date.now()}`;
const STAGGER_MS = Number(process.env.FUZZ_STAGGER_MS ?? "100");
const VERBOSE = process.env.FUZZ_VERBOSE === "1";

interface IterationResult {
	workerIndex: number;
	iteration: number;
	actorId: string;
	openMs: number;
	sleepPostMs: number;
	closeMs: number | null; // null if leaked
	closeCode?: number;
	closeReason?: string;
	leaked: boolean;
	error?: string;
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

async function waitForOpen(ws: WebSocket, timeoutMs: number): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;
	await new Promise<void>((resolve, reject) => {
		const t = setTimeout(() => reject(new Error("ws open timeout")), timeoutMs);
		ws.addEventListener(
			"open",
			() => {
				clearTimeout(t);
				resolve();
			},
			{ once: true },
		);
		ws.addEventListener(
			"error",
			() => {
				clearTimeout(t);
				reject(new Error("websocket error"));
			},
			{ once: true },
		);
		ws.addEventListener(
			"close",
			(event) => {
				clearTimeout(t);
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

async function runIteration(
	workerIndex: number,
	iteration: number,
): Promise<IterationResult> {
	const key = `${KEY_PREFIX}-w${workerIndex}-i${iteration}`;
	const client = createClient<typeof registry>({
		endpoint: ENDPOINT,
		namespace: NAMESPACE,
		token: TOKEN,
	});

	const handle = client.sleepCloseFuzz.getOrCreate([key]);
	const t0 = Date.now();
	const actorId = await handle.resolve();

	const wsUrl = buildWebSocketUrl(actorId);
	const ws = new WebSocket(wsUrl, ["rivet", "rivet_encoding.json"]);

	let closeCode: number | undefined;
	let closeReason: string | undefined;
	let closeAt = 0;
	const closePromise = new Promise<void>((resolve) => {
		ws.addEventListener(
			"close",
			(event) => {
				closeCode = event.code;
				closeReason = event.reason;
				closeAt = Date.now();
				resolve();
			},
			{ once: true },
		);
	});

	try {
		await waitForOpen(ws, 15_000);
	} catch (error) {
		try {
			ws.close();
		} catch {}
		return {
			workerIndex,
			iteration,
			actorId,
			openMs: Date.now() - t0,
			sleepPostMs: 0,
			closeMs: null,
			leaked: false,
			error: `open: ${formatError(error)}`,
		};
	}

	const openMs = Date.now() - t0;

	// Run for a few seconds with random jitter so sleep timing varies across iterations.
	const wait =
		WAIT_BEFORE_SLEEP_MS + Math.floor(Math.random() * WAIT_BEFORE_SLEEP_JITTER_MS);
	await sleep(wait);

	const tSleepStart = Date.now();
	let sleepPostStatus = 0;
	try {
		const response = await fetch(buildSleepUrl(actorId), {
			method: "POST",
			headers: {
				Authorization: TOKEN ? `Bearer ${TOKEN}` : "",
				"content-type": "application/json",
			},
			body: "{}",
		});
		sleepPostStatus = response.status;
		if (!response.ok) {
			const body = await response.text();
			throw new Error(`sleep POST status=${response.status} body=${body}`);
		}
	} catch (error) {
		try {
			ws.close();
		} catch {}
		return {
			workerIndex,
			iteration,
			actorId,
			openMs,
			sleepPostMs: Date.now() - tSleepStart,
			closeMs: null,
			leaked: false,
			error: `sleep-post: ${formatError(error)} status=${sleepPostStatus}`,
		};
	}
	const sleepPostMs = Date.now() - tSleepStart;

	// Race: close event vs leak threshold
	const leakTimeout = sleep(LEAK_THRESHOLD_MS).then(() => "timeout" as const);
	const result = await Promise.race([
		closePromise.then(() => "closed" as const),
		leakTimeout,
	]);

	if (result === "timeout") {
		// Leak: close did not arrive in time. Force-close client side and flag it.
		const stillOpen = ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING;
		try {
			ws.close(4000, "fuzz leak forced close");
		} catch {}
		return {
			workerIndex,
			iteration,
			actorId,
			openMs,
			sleepPostMs,
			closeMs: null,
			leaked: stillOpen,
			error: stillOpen ? undefined : "race lost but ws already closed",
		};
	}

	const closeMs = closeAt - tSleepStart;
	return {
		workerIndex,
		iteration,
		actorId,
		openMs,
		sleepPostMs,
		closeMs,
		closeCode,
		closeReason,
		leaked: false,
	};
}

async function runWorker(
	workerIndex: number,
	stopAt: number,
	results: IterationResult[],
): Promise<void> {
	if (workerIndex * STAGGER_MS > 0) {
		await sleep(workerIndex * STAGGER_MS);
	}
	let iteration = 0;
	while (Date.now() < stopAt) {
		iteration += 1;
		try {
			const result = await runIteration(workerIndex, iteration);
			results.push(result);
			if (result.leaked) {
				console.error(
					`[LEAK] worker=${workerIndex} iter=${iteration} actorId=${result.actorId} openMs=${result.openMs} sleepPostMs=${result.sleepPostMs} did NOT close within ${LEAK_THRESHOLD_MS}ms`,
				);
			} else if (result.error) {
				console.warn(
					`[err] worker=${workerIndex} iter=${iteration} actorId=${result.actorId} ${result.error}`,
				);
			} else if (VERBOSE) {
				console.log(
					`[ok] worker=${workerIndex} iter=${iteration} actorId=${result.actorId} closeMs=${result.closeMs} code=${result.closeCode} reason=${result.closeReason}`,
				);
			}
		} catch (error) {
			console.error(
				`[fatal-iter] worker=${workerIndex} iter=${iteration} ${formatError(error)}`,
			);
			results.push({
				workerIndex,
				iteration,
				actorId: "<unknown>",
				openMs: 0,
				sleepPostMs: 0,
				closeMs: null,
				leaked: false,
				error: formatError(error),
			});
		}
	}
}

function summarize(results: IterationResult[]) {
	const total = results.length;
	const leaks = results.filter((r) => r.leaked);
	const errors = results.filter((r) => r.error && !r.leaked);
	const ok = results.filter((r) => !r.leaked && !r.error);
	const closeMs = ok.map((r) => r.closeMs ?? 0).sort((a, b) => a - b);

	function pct(p: number): number {
		if (closeMs.length === 0) return 0;
		const i = Math.min(closeMs.length - 1, Math.floor((p / 100) * closeMs.length));
		return closeMs[i];
	}

	const avg =
		closeMs.length > 0
			? closeMs.reduce((s, x) => s + x, 0) / closeMs.length
			: 0;

	console.log(`\n[summary] ===========================`);
	console.log(`  total iterations: ${total}`);
	console.log(`  ok:               ${ok.length}`);
	console.log(`  errors:           ${errors.length}`);
	console.log(`  LEAKS:            ${leaks.length}`);
	if (ok.length > 0) {
		console.log(
			`  closeMs avg=${avg.toFixed(0)} p50=${pct(50)} p95=${pct(95)} p99=${pct(99)} max=${closeMs[closeMs.length - 1]}`,
		);
	}
	if (leaks.length > 0) {
		console.log(`\n[leaks] -------------------------`);
		for (const leak of leaks) {
			console.log(
				`  worker=${leak.workerIndex} iter=${leak.iteration} actorId=${leak.actorId} openMs=${leak.openMs} sleepPostMs=${leak.sleepPostMs}`,
			);
		}
	}
	if (errors.length > 0 && VERBOSE) {
		console.log(`\n[errors] ------------------------`);
		const byMsg = new Map<string, number>();
		for (const e of errors) {
			const msg = e.error ?? "<none>";
			byMsg.set(msg, (byMsg.get(msg) ?? 0) + 1);
		}
		for (const [msg, count] of byMsg) {
			console.log(`  x${count}  ${msg}`);
		}
	}
}

async function main() {
	if (!Number.isInteger(PARALLELISM) || PARALLELISM < 1) {
		throw new Error("FUZZ_PARALLELISM must be a positive integer");
	}

	console.log(
		`[fuzz] endpoint=${ENDPOINT} namespace=${NAMESPACE} parallelism=${PARALLELISM} durationMs=${DURATION_MS} waitBeforeSleepMs=${WAIT_BEFORE_SLEEP_MS}+jitter${WAIT_BEFORE_SLEEP_JITTER_MS} leakThresholdMs=${LEAK_THRESHOLD_MS} keyPrefix=${KEY_PREFIX}`,
	);

	const results: IterationResult[] = [];
	const stopAt = Date.now() + DURATION_MS;
	await Promise.all(
		Array.from({ length: PARALLELISM }, (_, i) => runWorker(i, stopAt, results)),
	);
	summarize(results);

	const leaks = results.filter((r) => r.leaked).length;
	process.exit(leaks > 0 ? 1 : 0);
}

main().catch((error) => {
	console.error(`[fatal] ${formatError(error)}`);
	process.exit(1);
});
