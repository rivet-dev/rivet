// Increment-only load test.
//
// Spawns N parallel workers. Each worker, on a fixed cadence:
//   1. opens a connection to a unique loadTestCounter actor
//   2. calls increment()
//   3. disposes the connection
//
// We do NOT impose a client-side deadline on the action. Instead we log a
// warning when a call crosses LOAD_TEST_SLOW_WARN_MS (default 1s) and let
// the call run to natural completion so the real rivetkit error
// (group/code/message) surfaces on failure. Faithfully prints whatever the
// client receives.
//
// Also polls the engine Prometheus metrics endpoint every interval and
// writes timestamped scrapes to a file so trends can be analyzed offline.
//
// Usage:
//   RIVET_ENDPOINT=http://127.0.0.1:6420 \
//   RIVET_SERVERLESS_URL=http://127.0.0.1:3000/api/rivet \
//   pnpm --filter kitchen-sink load-test

import { appendFileSync, openSync, closeSync, writeSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const SERVERLESS_URL = process.env.RIVET_SERVERLESS_URL;
const NAMESPACE =
	process.env.LOAD_TEST_NAMESPACE ??
	process.env.RIVET_NAMESPACE ??
	"default";
const TOKEN = process.env.LOAD_TEST_TOKEN ?? process.env.RIVET_TOKEN ?? "dev";
const POOL_NAME =
	process.env.LOAD_TEST_POOL ?? process.env.RIVET_POOL ?? "default";
const DURATION_MS = Number(process.env.LOAD_TEST_DURATION_MS ?? "300000");
const PARALLELISM = Number(process.env.LOAD_TEST_PARALLELISM ?? "10");
const INTERVAL_MS = Number(process.env.LOAD_TEST_INTERVAL_MS ?? "1000");
const SLOW_WARN_MS = Number(process.env.LOAD_TEST_SLOW_WARN_MS ?? "1000");
const SUMMARY_INTERVAL_MS = Number(
	process.env.LOAD_TEST_SUMMARY_INTERVAL_MS ?? "1000",
);
const METRICS_URL =
	process.env.LOAD_TEST_METRICS_URL ?? "http://127.0.0.1:6430/metrics";
const ENVOY_METRICS_URL =
	process.env.LOAD_TEST_ENVOY_METRICS_URL ?? "http://127.0.0.1:3000/metrics";
const METRICS_INTERVAL_MS = Number(
	process.env.LOAD_TEST_METRICS_INTERVAL_MS ?? "1000",
);
const METRICS_OUT =
	process.env.LOAD_TEST_METRICS_OUT ?? "/tmp/load-test-metrics.jsonl";
const ENVOY_METRICS_OUT =
	process.env.LOAD_TEST_ENVOY_METRICS_OUT ??
	"/tmp/load-test-envoy-metrics.jsonl";
const KEY_PREFIX =
	process.env.LOAD_TEST_KEY_PREFIX ?? `load-test-${Date.now()}`;

interface CallSample {
	workerIndex: number;
	attempt: number;
	startedAtMs: number;
	durationMs: number;
	ok: boolean;
	slow?: boolean;
	group?: string;
	code?: string;
	message?: string;
}

const samples: CallSample[] = [];
let successes = 0;
let failures = 0;
const errorCounts = new Map<string, number>();

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function describeError(error: unknown): {
	group: string;
	code: string;
	message: string;
} {
	if (error && typeof error === "object") {
		const e = error as Record<string, unknown>;
		const group = typeof e.group === "string" ? e.group : undefined;
		const code = typeof e.code === "string" ? e.code : undefined;
		if (group && code) {
			const message =
				typeof e.message === "string" ? e.message : String(error);
			return { group, code, message };
		}
		if (error instanceof Error) {
			return {
				group: "client",
				code: error.name,
				message: error.message,
			};
		}
	}
	return { group: "client", code: "unknown", message: String(error) };
}

function percentile(sorted: number[], p: number): number {
	if (sorted.length === 0) return 0;
	const rank = (p / 100) * (sorted.length - 1);
	const lo = Math.floor(rank);
	const hi = Math.ceil(rank);
	if (lo === hi) return sorted[lo];
	const frac = rank - lo;
	return sorted[lo] + (sorted[hi] - sorted[lo]) * frac;
}

function formatMs(value: number): string {
	return `${value.toFixed(1)}ms`;
}

async function triggerServerlessConfiguration() {
	if (!SERVERLESS_URL) return;
	const url = `${SERVERLESS_URL.replace(/\/$/, "")}/metadata`;
	console.log(`[configure] hitting ${url}`);
	const response = await fetch(url);
	console.log(`[configure] status=${response.status}`);
}

function recordSample(sample: CallSample) {
	samples.push(sample);
	if (sample.ok) {
		successes += 1;
	} else {
		failures += 1;
		const tag = `${sample.group}.${sample.code}`;
		errorCounts.set(tag, (errorCounts.get(tag) ?? 0) + 1);
	}

	const slowTag = sample.slow ? " slow=yes" : "";
	if (sample.ok) {
		console.log(
			`[call] worker=${sample.workerIndex} attempt=${sample.attempt} ok duration=${formatMs(sample.durationMs)}${slowTag}`,
		);
	} else {
		console.log(
			`[call] worker=${sample.workerIndex} attempt=${sample.attempt} fail duration=${formatMs(sample.durationMs)} group=${sample.group} code=${sample.code}${slowTag} message="${sample.message}"`,
		);
	}
}

function printSummary() {
	const total = successes + failures;
	if (total === 0) {
		console.log("[summary] (no samples yet)");
		return;
	}

	const okDurations = samples
		.filter((sample) => sample.ok)
		.map((sample) => sample.durationMs)
		.sort((a, b) => a - b);

	const p50 = percentile(okDurations, 50);
	const p95 = percentile(okDurations, 95);
	const p99 = percentile(okDurations, 99);
	const successRate = (successes / total) * 100;

	const topErrors = Array.from(errorCounts.entries())
		.sort((a, b) => b[1] - a[1])
		.slice(0, 4)
		.map(([code, n]) => `${code}=${n}`)
		.join(" ");

	console.log(
		`[summary] total=${total} ok=${successes} fail=${failures} success=${successRate.toFixed(1)}% p50=${formatMs(p50)} p95=${formatMs(p95)} p99=${formatMs(p99)}${topErrors ? ` errors=[${topErrors}]` : ""}`,
	);
}

async function runOneCall(workerIndex: number, attempt: number, key: string) {
	const client = createClient<typeof registry>({
		endpoint: ENDPOINT,
		namespace: NAMESPACE,
		token: TOKEN,
		poolName: POOL_NAME,
		disableMetadataLookup: true,
	});

	const startedAtMs = Date.now();
	const startedAtPerf = performance.now();

	const handle = client.loadTestCounter.getOrCreate([key]);
	const connection = handle.connect();

	let slow = false;
	const slowWarnTimer = setTimeout(() => {
		slow = true;
		console.warn(
			`[slow] worker=${workerIndex} attempt=${attempt} key=${key} crossed ${SLOW_WARN_MS}ms — still waiting`,
		);
	}, SLOW_WARN_MS);

	try {
		await connection.increment();
		clearTimeout(slowWarnTimer);
		recordSample({
			workerIndex,
			attempt,
			startedAtMs,
			durationMs: performance.now() - startedAtPerf,
			ok: true,
			slow,
		});
	} catch (error) {
		clearTimeout(slowWarnTimer);
		const { group, code, message } = describeError(error);
		recordSample({
			workerIndex,
			attempt,
			startedAtMs,
			durationMs: performance.now() - startedAtPerf,
			ok: false,
			group,
			code,
			message,
			slow,
		});
	} finally {
		await connection.dispose().catch(() => undefined);
		await client.dispose().catch(() => undefined);
	}
}

async function runWorker(workerIndex: number, stopAt: number) {
	const startDelayMs = (workerIndex * INTERVAL_MS) / PARALLELISM;
	if (startDelayMs > 0) await sleep(startDelayMs);

	let attempt = 0;
	while (Date.now() < stopAt) {
		const tickStartedAt = Date.now();
		attempt += 1;
		const key = `${KEY_PREFIX}-w${workerIndex}-a${attempt}`;

		await runOneCall(workerIndex, attempt, key);

		const elapsed = Date.now() - tickStartedAt;
		const wait = INTERVAL_MS - elapsed;
		if (wait > 0 && Date.now() + wait < stopAt) {
			await sleep(wait);
		}
	}
}

interface MetricsPoller {
	stop: () => void;
}

function pollMetricsTarget(url: string, outPath: string): MetricsPoller {
	const fd = openSync(outPath, "w");
	console.log(
		`[metrics] polling ${url} every ${METRICS_INTERVAL_MS}ms -> ${outPath}`,
	);

	let stopped = false;
	let timer: NodeJS.Timeout | undefined;

	const tick = async () => {
		if (stopped) return;
		const startedAt = Date.now();
		let status = 0;
		let body = "";
		let error: string | undefined;
		try {
			const response = await fetch(url, {
				signal: AbortSignal.timeout(2000),
			});
			status = response.status;
			body = await response.text();
		} catch (e) {
			error = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
		}

		if (stopped) return;

		const sample = {
			ts: startedAt,
			ms: Date.now() - startedAt,
			status,
			error,
			body,
		};
		writeSync(fd, `${JSON.stringify(sample)}\n`);
	};

	const loop = async () => {
		while (!stopped) {
			await tick();
			if (stopped) break;
			await new Promise<void>((resolve) => {
				timer = setTimeout(resolve, METRICS_INTERVAL_MS);
			});
		}
	};
	const loopPromise = loop();

	return {
		stop: () => {
			stopped = true;
			if (timer) clearTimeout(timer);
			loopPromise.finally(() => {
				try {
					closeSync(fd);
				} catch (_) {
					// already closed
				}
			});
		},
	};
}

function startMetricsPoller(): MetricsPoller {
	const enginePoller = pollMetricsTarget(METRICS_URL, METRICS_OUT);
	const envoyPoller = pollMetricsTarget(ENVOY_METRICS_URL, ENVOY_METRICS_OUT);
	return {
		stop: () => {
			enginePoller.stop();
			envoyPoller.stop();
		},
	};
}

async function main() {
	if (!Number.isInteger(PARALLELISM) || PARALLELISM < 1) {
		throw new Error("LOAD_TEST_PARALLELISM must be a positive integer");
	}

	console.log(
		`[start] endpoint=${ENDPOINT} namespace=${NAMESPACE} pool=${POOL_NAME} parallelism=${PARALLELISM} intervalMs=${INTERVAL_MS} slowWarnMs=${SLOW_WARN_MS} durationMs=${DURATION_MS} keyPrefix=${KEY_PREFIX} metricsUrl=${METRICS_URL}`,
	);

	await triggerServerlessConfiguration();

	const metricsPoller = startMetricsPoller();
	const stopAt = Date.now() + DURATION_MS;
	const summaryTimer = setInterval(printSummary, SUMMARY_INTERVAL_MS);

	try {
		await Promise.all(
			Array.from({ length: PARALLELISM }, (_, i) => runWorker(i, stopAt)),
		);
	} finally {
		clearInterval(summaryTimer);
		metricsPoller.stop();
	}

	console.log("[done] final summary:");
	printSummary();
	console.log("[done] error breakdown:");
	const sorted = Array.from(errorCounts.entries()).sort(
		(a, b) => b[1] - a[1],
	);
	for (const [code, n] of sorted) {
		console.log(`[done]   ${code}: ${n}`);
	}
}

main().catch((error: unknown) => {
	console.error(
		`[fatal] ${error instanceof Error ? `${error.name}: ${error.message}` : String(error)}`,
	);
	process.exit(1);
});
