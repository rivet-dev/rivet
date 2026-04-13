#!/usr/bin/env -S npx tsx

/**
 * Throughput benchmark. Creates a fresh testThroughput actor (50-table migration)
 * then calls increment in a loop, printing per-call latency.
 *
 * Usage:
 *   npx tsx scripts/bench-throughput.ts [count]
 */

const NAMESPACE = "kitchen-sink-7bq6-test-2-g5pg";
const TOKEN = "pk_6iY9qgm1ER09ks5hDHU67RFtPIoaJ0si4hie5VNq41NGhCDEzlexvh7vm08sSDXM";
const HOST = "https://api.staging.rivet.dev";
const COUNT = parseInt(process.argv[2] || "100", 10);

async function callAction(
	actorName: string,
	key: string[],
	action: string,
	args: unknown[] = [],
): Promise<unknown> {
	const params = new URLSearchParams({
		"rvt-method": "getOrCreate",
		"rvt-key": key.join(","),
		"rvt-token": TOKEN,
		"rvt-namespace": NAMESPACE,
		"rvt-runner": "default",
	});
	const actionUrl = `${HOST}/gateway/${actorName}/action/${action}?${params}`;
	const res = await fetch(actionUrl, {
		method: "POST",
		headers: { "Content-Type": "application/json", "x-rivet-encoding": "json" },
		body: JSON.stringify({ args }),
	});
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`${res.status}: ${text}`);
	}
	return (await res.json()).output;
}

function fmt(n: number): string {
	return `${n.toFixed(1)}ms`;
}

async function main(): Promise<void> {
	const uid = `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`;
	const key = [`throughput-${uid}`];

	console.log(`Throughput Benchmark`);
	console.log(`Host:      ${HOST}`);
	console.log(`Namespace: ${NAMESPACE}`);
	console.log(`Count:     ${COUNT}`);
	console.log();

	// Cold start: create actor (triggers 50-table migration)
	console.log("Creating actor (50-table migration)...");
	const coldStart = performance.now();
	const firstValue = await callAction("testThroughput", key, "increment");
	const coldMs = performance.now() - coldStart;
	console.log(`Cold start: ${fmt(coldMs)} (counter=${firstValue})\n`);

	// Increment loop
	console.log(`Running ${COUNT} increments...\n`);
	const times: number[] = [];
	const t0 = performance.now();

	for (let i = 0; i < COUNT; i++) {
		const start = performance.now();
		const value = await callAction("testThroughput", [`${key}-${i}`], "increment");
		const ms = performance.now() - start;
		times.push(ms);

		if ((i + 1) % 10 === 0 || i === COUNT - 1) {
			const elapsed = performance.now() - t0;
			const rps = ((i + 1) / elapsed) * 1000;
			process.stdout.write(
				`  [${String(i + 1).padStart(4)}/${COUNT}] ${fmt(ms).padStart(8)}  avg=${fmt(elapsed / (i + 1)).padStart(8)}  rps=${rps.toFixed(1).padStart(6)}  counter=${value}\n`,
			);
		}
	}

	const totalMs = performance.now() - t0;
	times.sort((a, b) => a - b);

	const avg = times.reduce((a, b) => a + b, 0) / times.length;
	const p50 = times[Math.floor(times.length * 0.5)];
	const p95 = times[Math.floor(times.length * 0.95)];
	const p99 = times[Math.floor(times.length * 0.99)];
	const min = times[0];
	const max = times[times.length - 1];
	const rps = (COUNT / totalMs) * 1000;

	console.log(`\n${"─".repeat(50)}`);
	console.log(`Cold start:  ${fmt(coldMs)}`);
	console.log(`Total:       ${fmt(totalMs)} (${COUNT} ops)`);
	console.log(`Throughput:  ${rps.toFixed(1)} rps`);
	console.log(`${"─".repeat(50)}`);
	console.log(`avg:  ${fmt(avg)}`);
	console.log(`p50:  ${fmt(p50)}`);
	console.log(`p95:  ${fmt(p95)}`);
	console.log(`p99:  ${fmt(p99)}`);
	console.log(`min:  ${fmt(min)}`);
	console.log(`max:  ${fmt(max)}`);
	console.log(`${"─".repeat(50)}`);
}

main().catch((err) => {
	console.error("Benchmark failed:", err);
	process.exit(1);
});
