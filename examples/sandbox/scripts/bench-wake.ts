#!/usr/bin/env -S npx tsx

/**
 * Wake latency benchmark for actors on Rivet Cloud.
 *
 * Measures:
 *   1. Cold start: getOrCreate a fresh actor + call no-op action
 *   2. Wake from sleep: put actor to sleep, then call no-op action again
 *
 * Usage:
 *   npx tsx scripts/bench-wake.ts <endpoint>
 *
 * Example:
 *   npx tsx scripts/bench-wake.ts \
 *     "https://my-ns:pk_token@api.staging.rivet.dev"
 */

const RAW_ENDPOINT = process.argv[2];
if (!RAW_ENDPOINT) {
	console.error("Usage: npx tsx scripts/bench-wake.ts <endpoint>");
	process.exit(1);
}

const url = new URL(RAW_ENDPOINT);
const NAMESPACE = url.username;
const TOKEN = url.password;
const HOST = `${url.protocol}//${url.host}`;

console.log(`Namespace: ${NAMESPACE}`);
console.log(`Host: ${HOST}\n`);

async function callAction(
	actorName: string,
	key: string[],
	action: string,
	args: unknown[] = [],
): Promise<{ output: unknown; ms: number }> {
	const params = new URLSearchParams({
		"rvt-method": "getOrCreate",
		"rvt-key": key.join(","),
		"rvt-token": TOKEN,
		"rvt-namespace": NAMESPACE,
		"rvt-runner": "default",
	});
	const actionUrl = `${HOST}/gateway/${actorName}/action/${action}?${params}`;
	const start = performance.now();
	const res = await fetch(actionUrl, {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
			"x-rivet-encoding": "json",
		},
		body: JSON.stringify({ args }),
	});
	const ms = performance.now() - start;
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`${res.status}: ${text}`);
	}
	const body = await res.json();
	return { output: body.output, ms };
}

function fmt(n: number): string {
	return `${n.toFixed(1)}ms`;
}

async function main(): Promise<void> {
	const ITERATIONS = 5;

	console.log(`Wake Latency Benchmark (${ITERATIONS} iterations)\n`);

	// Warmup: make sure the envoy is connected and ready
	console.log("Warming up...");
	const warmupKey = [`warmup-${Date.now()}`];
	await callAction("testWake", warmupKey, "noop");
	await callAction("testWake", warmupKey, "goToSleep");
	// Wait for sleep to take effect
	await new Promise((r) => setTimeout(r, 2000));
	await callAction("testWake", warmupKey, "noop");
	console.log("Warm.\n");

	const coldTimes: number[] = [];
	const wakeTimes: number[] = [];

	for (let i = 0; i < ITERATIONS; i++) {
		const uid = `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`;
		const key = [`bench-wake-${uid}`];

		// 1. Cold start: fresh actor creation + no-op
		const cold = await callAction("testWake", key, "noop");
		coldTimes.push(cold.ms);

		// 2. Put it to sleep
		await callAction("testWake", key, "goToSleep");

		// Wait for sleep to take effect
		await new Promise((r) => setTimeout(r, 2000));

		// 3. Wake: call no-op on sleeping actor
		const wake = await callAction("testWake", key, "noop");
		wakeTimes.push(wake.ms);

		console.log(
			`  [${i + 1}/${ITERATIONS}] cold=${fmt(cold.ms)}  wake=${fmt(wake.ms)}`,
		);
	}

	// Results
	const avg = (arr: number[]) => arr.reduce((a, b) => a + b, 0) / arr.length;
	const min = (arr: number[]) => Math.min(...arr);
	const max = (arr: number[]) => Math.max(...arr);
	const p50 = (arr: number[]) => {
		const sorted = [...arr].sort((a, b) => a - b);
		return sorted[Math.floor(sorted.length / 2)];
	};

	console.log("\n");
	const sep = "-".repeat(60);
	console.log(sep);
	console.log(
		`${"Metric".padEnd(30)}  ${"Cold Start".padStart(12)}  ${"Wake".padStart(12)}`,
	);
	console.log(sep);
	console.log(
		`${"avg".padEnd(30)}  ${fmt(avg(coldTimes)).padStart(12)}  ${fmt(avg(wakeTimes)).padStart(12)}`,
	);
	console.log(
		`${"p50".padEnd(30)}  ${fmt(p50(coldTimes)).padStart(12)}  ${fmt(p50(wakeTimes)).padStart(12)}`,
	);
	console.log(
		`${"min".padEnd(30)}  ${fmt(min(coldTimes)).padStart(12)}  ${fmt(min(wakeTimes)).padStart(12)}`,
	);
	console.log(
		`${"max".padEnd(30)}  ${fmt(max(coldTimes)).padStart(12)}  ${fmt(max(wakeTimes)).padStart(12)}`,
	);
	console.log(sep);
}

main().catch((err) => {
	console.error("Benchmark failed:", err);
	process.exit(1);
});
