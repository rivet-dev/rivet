#!/usr/bin/env -S npx tsx

/**
 * SQLite benchmark for the sandbox deployed on Rivet Cloud.
 *
 * Uses the Rivet gateway HTTP API with rvt-* query parameters.
 *
 * Usage:
 *   npx tsx scripts/bench-sqlite.ts <endpoint>
 *
 * Example:
 *   npx tsx scripts/bench-sqlite.ts \
 *     "https://my-ns:pk_token@api.staging.rivet.dev"
 */

const RAW_ENDPOINT = process.argv[2];
if (!RAW_ENDPOINT) {
	console.error("Usage: npx tsx scripts/bench-sqlite.ts <endpoint>");
	process.exit(1);
}

// Parse endpoint: https://namespace:token@host
const url = new URL(RAW_ENDPOINT);
const NAMESPACE = url.username;
const TOKEN = url.password;
const HOST = `${url.protocol}//${url.host}`;

console.log(`Namespace: ${NAMESPACE}`);
console.log(`Host: ${HOST}\n`);

// ── Helpers ────────────────────────────────────────────────────────

interface TimedResult<T> {
	result: T;
	ms: number;
}

async function timed<T>(fn: () => Promise<T>): Promise<TimedResult<T>> {
	const start = performance.now();
	const result = await fn();
	return { result, ms: performance.now() - start };
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
		"rvt-token": TOKEN,
		"rvt-namespace": NAMESPACE,
		"rvt-runner": "default",
	});
	const actionUrl = `${HOST}/gateway/${actorName}/action/${action}?${params}`;
	const res = await fetch(actionUrl, {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
			"x-rivet-encoding": "json",
		},
		body: JSON.stringify({ args }),
	});
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`${res.status}: ${text}`);
	}
	const body = await res.json();
	return body.output;
}

function fmt(n: number): string {
	return `${n.toFixed(1)}ms`;
}

function perOp(total: number, count: number): string {
	return `${(total / count).toFixed(3)}ms/op`;
}

interface BenchEntry {
	name: string;
	ms: number;
	detail?: string;
}

function printTable(entries: BenchEntry[]): void {
	const nameW = Math.max(50, ...entries.map((e) => e.name.length));
	const sep = "-".repeat(nameW + 35);
	console.log(sep);
	console.log(
		`${"Benchmark".padEnd(nameW)}  ${"Time".padStart(14)}  ${"Detail".padEnd(20)}`,
	);
	console.log(sep);
	for (const e of entries) {
		console.log(
			`${e.name.padEnd(nameW)}  ${fmt(e.ms).padStart(14)}  ${(e.detail ?? "").padEnd(20)}`,
		);
	}
	console.log(sep);
}

// ── Benchmarks ─────────────────────────────────────────────────────

async function main(): Promise<void> {
	console.log(`SQLite Benchmark\n`);

	const entries: BenchEntry[] = [];
	const uid = () => `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`;

	// ── Warmup ──
	console.log("Warming up (creating a counter actor)...");
	await callAction("testCounter", [`warmup-${uid()}`], "increment", [1]);
	console.log("Warm.\n");

	// ── 1. Cold SQLite actor creation ──
	console.log("[1] Cold SQLite actor create (sqliteRawActor)...");
	try {
		const key = [`bench-cold-${uid()}`];
		const { ms: coldMs } = await timed(() =>
			callAction("sqliteRawActor", key, "getTodos"),
		);
		entries.push({
			name: "Cold actor create + migrate + getTodos",
			ms: coldMs,
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({ name: "Cold actor create + migrate + getTodos", ms: -1, detail: "FAILED" });
	}

	// ── 2. Sequential inserts ──
	for (const n of [1, 10, 50]) {
		console.log(`[2] Insert x${n}...`);
		try {
			const key = [`bench-ins-${n}-${uid()}`];
			// Create actor
			await callAction("sqliteRawActor", key, "getTodos");

			const { ms: insertMs } = await timed(async () => {
				for (let i = 0; i < n; i++) {
					await callAction("sqliteRawActor", key, "addTodo", [`todo-${i}`]);
				}
			});
			entries.push({
				name: `Insert x${n} (sequential actions)`,
				ms: insertMs,
				detail: perOp(insertMs, n),
			});
		} catch (err) {
			console.log(`  FAILED: ${err}`);
			entries.push({ name: `Insert x${n}`, ms: -1, detail: "FAILED" });
		}
	}

	// ── 3. Read after writes ──
	console.log("[3] Read after writes...");
	try {
		const key = [`bench-read-${uid()}`];
		for (let i = 0; i < 50; i++) {
			await callAction("sqliteRawActor", key, "addTodo", [`rd-${i}`]);
		}
		const { ms: readMs, result } = await timed(() =>
			callAction("sqliteRawActor", key, "getTodos"),
		);
		const count = Array.isArray(result) ? result.length : "?";
		entries.push({
			name: `Read 50 todos (getTodos)`,
			ms: readMs,
			detail: `${count} rows`,
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({ name: "Read 50 todos", ms: -1, detail: "FAILED" });
	}

	// ── 4. Toggle (update) ──
	console.log("[4] Toggle (update) x20...");
	try {
		const key = [`bench-toggle-${uid()}`];
		for (let i = 0; i < 20; i++) {
			await callAction("sqliteRawActor", key, "addTodo", [`t-${i}`]);
		}
		const { ms: toggleMs } = await timed(async () => {
			for (let i = 1; i <= 20; i++) {
				await callAction("sqliteRawActor", key, "toggleTodo", [i]);
			}
		});
		entries.push({
			name: `Toggle x20 (update)`,
			ms: toggleMs,
			detail: perOp(toggleMs, 20),
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({ name: "Toggle x20", ms: -1, detail: "FAILED" });
	}

	// ── 5. Delete ──
	console.log("[5] Delete x20...");
	try {
		const key = [`bench-del-${uid()}`];
		for (let i = 0; i < 20; i++) {
			await callAction("sqliteRawActor", key, "addTodo", [`d-${i}`]);
		}
		const { ms: deleteMs } = await timed(async () => {
			for (let i = 1; i <= 20; i++) {
				await callAction("sqliteRawActor", key, "deleteTodo", [i]);
			}
		});
		entries.push({
			name: `Delete x20`,
			ms: deleteMs,
			detail: perOp(deleteMs, 20),
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({ name: "Delete x20", ms: -1, detail: "FAILED" });
	}

	// ── 6. Complex 20-query load test ──
	console.log("[6] testSqliteLoad: 20-query complex workload...");
	try {
		const key = [`bench-load-${uid()}`];
		const { ms: loadMs, result } = await timed(() =>
			callAction("testSqliteLoad", key, "runLoadTest"),
		);
		const queriesRun =
			result && typeof result === "object" && "queriesRun" in result
				? (result as { queriesRun: number }).queriesRun
				: "?";
		entries.push({
			name: `Complex load test (${queriesRun} queries)`,
			ms: loadMs,
			detail: `${queriesRun} queries`,
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({
			name: "Complex load test",
			ms: -1,
			detail: "FAILED",
		});
	}

	// ── 7. Concurrent actor creation ──
	console.log("[7] Concurrent 5 actors...");
	try {
		const { ms: concMs } = await timed(async () => {
			await Promise.all(
				Array.from({ length: 5 }, (_, i) =>
					callAction(
						"sqliteRawActor",
						[`bench-conc-${uid()}-${i}`],
						"addTodo",
						[`concurrent-${i}`],
					),
				),
			);
		});
		entries.push({
			name: `Concurrent 5 actor create+insert`,
			ms: concMs,
			detail: perOp(concMs, 5),
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({
			name: "Concurrent 5 actor create+insert",
			ms: -1,
			detail: "FAILED",
		});
	}

	// ── 8. Rapid-fire on warm actor ──
	console.log("[8] Rapid-fire x50 on warm actor...");
	try {
		const key = [`bench-rapid-${uid()}`];
		await callAction("sqliteRawActor", key, "getTodos");

		const n = 50;
		const { ms: rapidMs } = await timed(async () => {
			for (let i = 0; i < n; i++) {
				await callAction("sqliteRawActor", key, "addTodo", [`r-${i}`]);
			}
		});
		entries.push({
			name: `Rapid-fire x${n} inserts (warm actor)`,
			ms: rapidMs,
			detail: perOp(rapidMs, n),
		});
	} catch (err) {
		console.log(`  FAILED: ${err}`);
		entries.push({ name: "Rapid-fire x50", ms: -1, detail: "FAILED" });
	}

	// ── Results ──

	console.log("\n");
	printTable(entries);

	const passed = entries.filter((e) => e.ms > 0);
	const failed = entries.filter((e) => e.ms <= 0);
	const totalMs = passed.reduce((sum, e) => sum + e.ms, 0);
	console.log(`\nTotal: ${fmt(totalMs)}`);
	console.log(`Passed: ${passed.length}, Failed: ${failed.length}`);
}

main().catch((err) => {
	console.error("Benchmark failed:", err);
	process.exit(1);
});
