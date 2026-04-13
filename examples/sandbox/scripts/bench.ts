#!/usr/bin/env -S npx tsx

/**
 * Unified benchmark for Rivet Cloud actors.
 *
 * Measures baseline RTT via no-op on a warm actor and subtracts it from
 * all measurements to show server-side processing time.
 *
 * Usage:
 *   npx tsx scripts/bench.ts <endpoint> [--filter <pattern>]
 *
 * Examples:
 *   npx tsx scripts/bench.ts "https://ns:token@api.staging.rivet.dev"
 *   npx tsx scripts/bench.ts "https://ns:token@api.staging.rivet.dev" --filter wake
 *   npx tsx scripts/bench.ts "https://ns:token@api.staging.rivet.dev" --filter sqlite
 */

// ── Args ──────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const RAW_ENDPOINT = args.find((a) => !a.startsWith("--"));
const filterIdx = args.indexOf("--filter");
const FILTER = filterIdx >= 0 ? args[filterIdx + 1]?.toLowerCase() : undefined;

if (!RAW_ENDPOINT) {
	console.error("Usage: npx tsx scripts/bench.ts <endpoint> [--filter <pattern>]");
	process.exit(1);
}

const url = new URL(RAW_ENDPOINT);
const NAMESPACE = url.username;
const TOKEN = url.password;
const HOST = `${url.protocol}//${url.host}`;

// ── HTTP helpers ──────────────────────────────────────────────────

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

async function timed<T>(fn: () => Promise<T>): Promise<{ result: T; ms: number }> {
	const start = performance.now();
	const result = await fn();
	return { result, ms: performance.now() - start };
}

// ── Baseline RTT ──────────────────────────────────────────────────

async function measureBaseline(n = 20): Promise<{ baselineMs: number; baselineKey: string[] }> {
	const baselineKey = [`baseline-${uid()}`];

	// Warm up (discard first 3 for DNS/connection/cold start)
	await callAction("counter", baselineKey, "noop");
	await callAction("counter", baselineKey, "noop");
	await callAction("counter", baselineKey, "noop");

	const times: number[] = [];
	for (let i = 0; i < n; i++) {
		const start = performance.now();
		await callAction("counter", baselineKey, "noop");
		times.push(performance.now() - start);
	}
	times.sort((a, b) => a - b);
	return { baselineMs: times[Math.floor(times.length / 2)], baselineKey };
}

// ── Result table ──────────────────────────────────────────────────

interface BenchEntry {
	group: string;
	name: string;
	e2eMs: number;
	serverMs: number | null;
	perOpMs: number | null;
	failed?: boolean;
	failReason?: string;
}

function fmt(n: number): string {
	return `${n.toFixed(1)}ms`;
}

function printTable(entries: BenchEntry[], baselineMs: number): void {
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

// ── Bench definitions ─────────────────────────────────────────────

type BenchFn = () => Promise<BenchEntry>;

function uid(): string {
	return `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`;
}

function shouldRun(group: string, name: string): boolean {
	if (!FILTER) return true;
	return `${group} ${name}`.toLowerCase().includes(FILTER);
}

function benchLatency(): BenchFn[] {
	const group = "Latency";
	const benches: BenchFn[] = [];

	if (shouldRun(group, "HTTP ping (health endpoint)")) {
		benches.push(async () => {
			const healthUrl = `${HOST}/`;
			// Warmup
			await fetch(healthUrl);
			await fetch(healthUrl);
			const times: number[] = [];
			for (let i = 0; i < 10; i++) {
				const start = performance.now();
				await fetch(healthUrl);
				times.push(performance.now() - start);
			}
			times.sort((a, b) => a - b);
			const ms = times[Math.floor(times.length / 2)];
			return { group, name: "HTTP ping (health endpoint)", e2eMs: ms, serverMs: null, perOpMs: null };
		});
	}

	if (shouldRun(group, "Action ping (warm actor)")) {
		benches.push(async () => {
			const key = [`bench-ping-${uid()}`];
			await callAction("counter", key, "noop");
			const times: number[] = [];
			for (let i = 0; i < 10; i++) {
				const { ms } = await timed(() => callAction("counter", key, "noop"));
				times.push(ms);
			}
			times.sort((a, b) => a - b);
			const ms = times[Math.floor(times.length / 2)];
			return { group, name: "Action ping (warm actor)", e2eMs: ms, serverMs: null, perOpMs: null };
		});
	}

	if (shouldRun(group, "Cold start (fresh actor)")) {
		benches.push(async () => {
			const key = [`bench-cold-${uid()}`];
			const { ms } = await timed(() => callAction("counter", key, "noop"));
			return { group, name: "Cold start (fresh actor)", e2eMs: ms, serverMs: null, perOpMs: null };
		});
	}

	if (shouldRun(group, "Wake from sleep")) {
		benches.push(async () => {
			const key = [`bench-wake-${uid()}`];
			await callAction("counter", key, "noop");
			await callAction("counter", key, "goToSleep");
			await new Promise((r) => setTimeout(r, 2000));
			const { ms } = await timed(() => callAction("counter", key, "noop"));
			return { group, name: "Wake from sleep", e2eMs: ms, serverMs: null, perOpMs: null };
		});
	}

	return benches;
}

function benchSqlite(): BenchFn[] {
	const group = "SQLite";
	const benches: BenchFn[] = [];

	const sqliteBenches: { name: string; action: string; args: unknown[] }[] = [
		{ name: "Insert single x10", action: "insertSingle", args: [10] },
		{ name: "Insert single x100", action: "insertSingle", args: [100] },
		{ name: "Insert single x1000", action: "insertSingle", args: [1000] },
		{ name: "Insert single x10000", action: "insertSingle", args: [10000] },
		{ name: "Insert TX x1", action: "insertTx", args: [1] },
		{ name: "Insert TX x10", action: "insertTx", args: [10] },
		{ name: "Insert TX x10000", action: "insertTx", args: [10000] },
		{ name: "Insert batch x10", action: "insertBatch", args: [10] },
		{ name: "Point read x100", action: "pointRead", args: [100] },
		{ name: "Full scan (500 rows)", action: "fullScan", args: [500] },
		{ name: "Range scan indexed", action: "rangeScanIndexed", args: [] },
		{ name: "Range scan unindexed", action: "rangeScanUnindexed", args: [] },
		{ name: "Bulk update", action: "bulkUpdate", args: [] },
		{ name: "Bulk delete", action: "bulkDelete", args: [] },
		{ name: "Hot row updates x100", action: "hotRowUpdates", args: [100] },
		{ name: "Hot row updates x10000", action: "hotRowUpdates", args: [10000] },
		{ name: "VACUUM after delete", action: "vacuumAfterDelete", args: [] },
		{ name: "Large payload insert (32KB x20)", action: "largePayloadInsert", args: [20] },
		{ name: "Mixed OLTP x1", action: "mixedOltp", args: [] },
		{ name: "JSON extract query", action: "jsonInsertAndQuery", args: [] },
		{ name: "JSON each aggregation", action: "jsonEachAgg", args: [] },
		{ name: "Complex: aggregation", action: "complexAggregation", args: [] },
		{ name: "Complex: subquery", action: "complexSubquery", args: [] },
		{ name: "Complex: join (200 rows)", action: "complexJoin", args: [] },
		{ name: "Complex: CTE + window functions", action: "complexCteWindow", args: [] },
		{ name: "Migration (50 tables)", action: "migrationTables", args: [50] },
	];

	for (const b of sqliteBenches) {
		if (!shouldRun(group, b.name)) continue;
		benches.push(async () => {
			const key = [`bench-sql-${uid()}`];
			try {
				// Create actor first (cold start not measured)
				await callAction("testSqliteBench", key, "noop");
				const { result, ms: e2eMs } = await timed(() =>
					callAction("testSqliteBench", key, b.action, b.args),
				);
				const r = result as { ms?: number; ops?: number; [k: string]: unknown };
				const serverMs = r.ms ?? null;
				const perOpMs = (serverMs != null && r.ops) ? serverMs / r.ops : null;
				return { group, name: b.name, e2eMs, serverMs, perOpMs };
			} catch (err) {
				return { group, name: b.name, e2eMs: -1, serverMs: null, perOpMs: null, failed: true, failReason: String(err).slice(0, 80) };
			}
		});
	}

	// Concurrent actors
	if (shouldRun(group, "Concurrent 5 actors")) {
		benches.push(async () => {
			const { ms: wallMs } = await timed(async () => {
				await Promise.all(
					Array.from({ length: 5 }, (_, i) =>
						callAction("testSqliteBench", [`bench-conc-${uid()}-${i}`], "insertSingle", [10]),
					),
				);
			});
			return { group, name: "Concurrent 5 actors wall time", e2eMs: wallMs, serverMs: null, perOpMs: null };
		});
		benches.push(async () => {
			const times: number[] = [];
			await Promise.all(
				Array.from({ length: 5 }, async (_, i) => {
					const { ms } = await timed(() =>
						callAction("testSqliteBench", [`bench-conc2-${uid()}-${i}`], "insertSingle", [10]),
					);
					times.push(ms);
				}),
			);
			const avg = times.reduce((a, b) => a + b, 0) / times.length;
			return { group, name: "Concurrent 5 actors (per-actor)", e2eMs: avg, serverMs: null, perOpMs: null };
		});
	}

	return benches;
}

// ── Main ──────────────────────────────────────────────────────────

async function main(): Promise<void> {
	console.log(`Rivet Actor Benchmark`);
	console.log(`Namespace: ${NAMESPACE}`);
	console.log(`Host:      ${HOST}`);
	if (FILTER) console.log(`Filter:    ${FILTER}`);
	console.log();

	// Warmup: create an actor so envoy is connected
	console.log("Warming up envoy...");
	await callAction("counter", [`warmup-${uid()}`], "noop");
	console.log("Ready.\n");

	// Measure baseline RTT via no-op on a warm actor
	console.log("Measuring baseline RTT (no-op on warm actor)...");
	const { baselineMs } = await measureBaseline(20);
	console.log(`Baseline RTT (median of 20): ${fmt(baselineMs)}\n`);

	// Collect all benches
	const allBenches: BenchFn[] = [...benchLatency(), ...benchSqlite()];

	if (allBenches.length === 0) {
		console.log("No benchmarks matched the filter.");
		return;
	}

	console.log(`Running ${allBenches.length} benchmarks...\n`);

	const entries: BenchEntry[] = [];
	for (let i = 0; i < allBenches.length; i++) {
		const entry = await allBenches[i]();
		entries.push(entry);
		const status = entry.failed ? "FAILED" : fmt(entry.e2eMs);
		process.stdout.write(`  [${String(i + 1).padStart(2)}/${allBenches.length}] ${entry.name.padEnd(40)} ${status}\n`);
	}

	printTable(entries, baselineMs);

	const passed = entries.filter((e) => !e.failed);
	const failed = entries.filter((e) => e.failed);
	console.log(`\nBaseline RTT: ${fmt(baselineMs)}`);
	console.log(`Passed: ${passed.length}, Failed: ${failed.length}`);
}

main().catch((err) => {
	console.error("Benchmark failed:", err);
	process.exit(1);
});
