/**
 * Server-side benchmark runner. Executes all benchmarks from within the
 * Cloud Run instance so network hops stay inside the datacenter.
 */

import type { Registry } from "rivetkit";

interface BenchEntry {
	group: string;
	name: string;
	e2eMs: number;
	serverMs: number | null;
	perOpMs: number | null;
	failed?: boolean;
	failReason?: string;
}

function uid(): string {
	return `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`;
}

export async function runBenchmarks(
	registry: Registry<any>,
	filter?: string | null,
): Promise<{ baselineMs: number; entries: BenchEntry[] }> {
	const config = registry.parseConfig();
	const endpoint = config.endpoint;
	const namespace = config.namespace;
	const token = config.token;

	if (!endpoint || !namespace) {
		throw new Error("Registry has no endpoint/namespace configured");
	}

	async function callAction(
		actorName: string,
		key: string[],
		action: string,
		args: unknown[] = [],
	): Promise<unknown> {
		const params = new URLSearchParams({
			"rvt-method": "getOrCreate",
			"rvt-key": key.join(","),
			"rvt-namespace": namespace,
			"rvt-runner": "default",
		});
		if (token) params.set("rvt-token", token);
		const actionUrl = `${endpoint}/gateway/${actorName}/action/${action}?${params}`;
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

	function shouldRun(group: string, name: string): boolean {
		if (!filter) return true;
		return `${group} ${name}`.toLowerCase().includes(filter.toLowerCase());
	}

	// Warmup
	await callAction("counter", [`warmup-${uid()}`], "noop");

	// Baseline: no-op on warm actor
	const baselineKey = [`baseline-${uid()}`];
	await callAction("counter", baselineKey, "noop");
	await callAction("counter", baselineKey, "noop");
	await callAction("counter", baselineKey, "noop");

	const baselineTimes: number[] = [];
	for (let i = 0; i < 20; i++) {
		const start = performance.now();
		await callAction("counter", baselineKey, "noop");
		baselineTimes.push(performance.now() - start);
	}
	baselineTimes.sort((a, b) => a - b);
	const baselineMs = baselineTimes[Math.floor(baselineTimes.length / 2)];

	const entries: BenchEntry[] = [];

	// ── Latency ───────────────────────────────────────────────────

	const latencyGroup = "Latency";

	if (shouldRun(latencyGroup, "Action ping (warm actor)")) {
		const key = [`bench-ping-${uid()}`];
		await callAction("counter", key, "noop");
		const times: number[] = [];
		for (let i = 0; i < 10; i++) {
			const { ms } = await timed(() => callAction("counter", key, "noop"));
			times.push(ms);
		}
		times.sort((a, b) => a - b);
		entries.push({ group: latencyGroup, name: "Action ping (warm actor)", e2eMs: times[Math.floor(times.length / 2)], serverMs: null, perOpMs: null });
	}

	if (shouldRun(latencyGroup, "Cold start (fresh actor)")) {
		const key = [`bench-cold-${uid()}`];
		const { ms } = await timed(() => callAction("counter", key, "noop"));
		entries.push({ group: latencyGroup, name: "Cold start (fresh actor)", e2eMs: ms, serverMs: null, perOpMs: null });
	}

	if (shouldRun(latencyGroup, "Wake from sleep")) {
		const key = [`bench-wake-${uid()}`];
		await callAction("counter", key, "noop");
		await callAction("counter", key, "goToSleep");
		await new Promise((r) => setTimeout(r, 2000));
		const { ms } = await timed(() => callAction("counter", key, "noop"));
		entries.push({ group: latencyGroup, name: "Wake from sleep", e2eMs: ms, serverMs: null, perOpMs: null });
	}

	// ── SQLite ────────────────────────────────────────────────────

	const sqliteGroup = "SQLite";
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
		if (!shouldRun(sqliteGroup, b.name)) continue;
		const key = [`bench-sql-${uid()}`];
		try {
			await callAction("testSqliteBench", key, "noop");
			const { result, ms: e2eMs } = await timed(() =>
				callAction("testSqliteBench", key, b.action, b.args),
			);
			const r = result as { ms?: number; ops?: number; [k: string]: unknown };
			const serverMs = r.ms ?? null;
			const perOpMs = (serverMs != null && r.ops) ? serverMs / r.ops : null;
			entries.push({ group: sqliteGroup, name: b.name, e2eMs, serverMs, perOpMs });
		} catch (err) {
			entries.push({ group: sqliteGroup, name: b.name, e2eMs: -1, serverMs: null, perOpMs: null, failed: true, failReason: String(err).slice(0, 120) });
		}
	}

	// Concurrent actors
	if (shouldRun(sqliteGroup, "Concurrent 5 actors")) {
		const { ms: wallMs } = await timed(async () => {
			await Promise.all(
				Array.from({ length: 5 }, (_, i) =>
					callAction("testSqliteBench", [`bench-conc-${uid()}-${i}`], "insertSingle", [10]),
				),
			);
		});
		entries.push({ group: sqliteGroup, name: "Concurrent 5 actors wall time", e2eMs: wallMs, serverMs: null, perOpMs: null });

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
		entries.push({ group: sqliteGroup, name: "Concurrent 5 actors (per-actor)", e2eMs: avg, serverMs: null, perOpMs: null });
	}

	return { baselineMs, entries };
}
