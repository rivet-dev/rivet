#!/usr/bin/env -S npx tsx

/**
 * Remote benchmark runner. Calls the /api/bench endpoint on Cloud Run
 * so all actor calls happen within the datacenter.
 *
 * Also measures local baseline RTT for comparison.
 *
 * Usage:
 *   npx tsx scripts/bench-remote.ts <cloud-run-url> [--filter <pattern>]
 *
 * Example:
 *   npx tsx scripts/bench-remote.ts https://kitchen-sink-staging-676044580344.us-east4.run.app
 *   npx tsx scripts/bench-remote.ts https://kitchen-sink-staging-676044580344.us-east4.run.app --filter wake
 */

const args = process.argv.slice(2);
const CLOUD_RUN_URL = args.find((a) => !a.startsWith("--"))?.replace(/\/$/, "");
const filterIdx = args.indexOf("--filter");
const FILTER = filterIdx >= 0 ? args[filterIdx + 1] : undefined;

if (!CLOUD_RUN_URL) {
	console.error("Usage: npx tsx scripts/bench-remote.ts <cloud-run-url> [--filter <pattern>]");
	process.exit(1);
}

function fmt(n: number): string {
	return `${n.toFixed(1)}ms`;
}

interface BenchEntry {
	group: string;
	name: string;
	e2eMs: number;
	serverMs: number | null;
	perOpMs: number | null;
	failed?: boolean;
	failReason?: string;
}

function printTable(entries: BenchEntry[], baselineMs: number, localRttMs: number): void {
	const nameW = Math.max(40, ...entries.map((e) => e.name.length));
	const sep = "─".repeat(nameW + 46);

	console.log(`\n┌${sep}┐`);
	console.log(
		`│ ${"Benchmark".padEnd(nameW)}  ${"E2E".padStart(10)}  ${"Server".padStart(10)}  ${"Per-Op".padStart(10)}  ${"RTT".padStart(8)} │`,
	);
	console.log(`├${sep}┤`);

	let currentGroup = "";
	for (const e of entries) {
		if (e.group !== currentGroup) {
			if (currentGroup) console.log(`├${sep}┤`);
			console.log(`│ ${`── ${e.group} ──`.padEnd(nameW + 44)} │`);
			currentGroup = e.group;
		}
		if (e.failed) {
			console.log(`│ ${e.name.padEnd(nameW)}  ${"FAILED".padStart(10)}  ${"".padStart(10)}  ${"".padStart(10)}  ${"".padStart(8)} │`);
		} else {
			const serverStr = e.serverMs != null ? fmt(e.serverMs) : fmt(Math.max(0, e.e2eMs - baselineMs));
			const perOpStr = e.perOpMs != null ? fmt(e.perOpMs) : "";
			const rtt = e.serverMs != null ? fmt(e.e2eMs - e.serverMs) : fmt(baselineMs);
			console.log(
				`│ ${e.name.padEnd(nameW)}  ${fmt(e.e2eMs).padStart(10)}  ${serverStr.padStart(10)}  ${perOpStr.padStart(10)}  ${rtt.padStart(8)} │`,
			);
		}
	}
	console.log(`└${sep}┘`);
}

async function main(): Promise<void> {
	console.log(`Rivet Actor Benchmark (server-side)`);
	console.log(`Cloud Run: ${CLOUD_RUN_URL}`);
	if (FILTER) console.log(`Filter:    ${FILTER}`);
	console.log();

	// Measure local RTT to Cloud Run for reference
	console.log("Measuring local RTT to Cloud Run...");
	await fetch(`${CLOUD_RUN_URL}/`);
	await fetch(`${CLOUD_RUN_URL}/`);
	const localTimes: number[] = [];
	for (let i = 0; i < 10; i++) {
		const start = performance.now();
		await fetch(`${CLOUD_RUN_URL}/`);
		localTimes.push(performance.now() - start);
	}
	localTimes.sort((a, b) => a - b);
	const localRttMs = localTimes[Math.floor(localTimes.length / 2)];
	console.log(`Local RTT to Cloud Run (median of 10): ${fmt(localRttMs)}\n`);

	// Call server-side bench endpoint
	const benchUrl = FILTER
		? `${CLOUD_RUN_URL}/api/bench?filter=${encodeURIComponent(FILTER)}`
		: `${CLOUD_RUN_URL}/api/bench`;

	console.log("Running benchmarks server-side...");
	console.log("(This may take a few minutes)\n");

	const res = await fetch(benchUrl, { signal: AbortSignal.timeout(600_000) });
	if (!res.ok) {
		const text = await res.text();
		console.error(`Bench endpoint failed: ${res.status}: ${text}`);
		process.exit(1);
	}

	const { baselineMs, entries } = (await res.json()) as {
		baselineMs: number;
		entries: BenchEntry[];
	};

	console.log(`Server-side baseline RTT (datacenter): ${fmt(baselineMs)}`);
	console.log(`Local RTT (client → Cloud Run):        ${fmt(localRttMs)}`);

	printTable(entries, baselineMs, localRttMs);

	const passed = entries.filter((e: BenchEntry) => !e.failed);
	const failed = entries.filter((e: BenchEntry) => e.failed);
	console.log(`\nServer baseline RTT: ${fmt(baselineMs)}`);
	console.log(`Local RTT: ${fmt(localRttMs)}`);
	console.log(`Passed: ${passed.length}, Failed: ${failed.length}`);
}

main().catch((err) => {
	console.error("Benchmark failed:", err);
	process.exit(1);
});
