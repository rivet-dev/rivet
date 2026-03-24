#!/usr/bin/env -S npx tsx

import { setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { sqliteBench } from "../src/actors/sqlite-bench.ts";

const registry = setup({ use: { sqliteBench } });
type R = typeof registry;

async function main() {
	const endpoint = process.env.RIVET_ENDPOINT || "http://127.0.0.1:6420";
	registry.start();
	const client = createClient<R>({ endpoint });
	await new Promise(r => setTimeout(r, 5000));

	function fresh() {
		return client.sqliteBench.getOrCreate(["diag-" + Math.random().toString(36).slice(2)]);
	}

	// Cold reads: fresh actor every time (like the bench does)
	console.log("=== Cold actor point reads ===");
	for (const n of [1, 10, 100]) {
		const a = fresh();
		const r = await a.benchPointRead(n);
		console.log(`Fresh actor point read x${n}: ${r.elapsedMs.toFixed(1)}ms (${(r.elapsedMs / n).toFixed(3)}ms/op)`);
	}

	// Warm reads: reuse same actor
	console.log("\n=== Warm actor point reads ===");
	const a = fresh();
	await a.benchInsertTransaction(1000);
	for (const n of [1, 10, 100, 1000]) {
		const r = await a.benchPointRead(n);
		console.log(`Warm actor point read x${n}: ${r.elapsedMs.toFixed(1)}ms (${(r.elapsedMs / n).toFixed(3)}ms/op)`);
	}

	// Cold TX: fresh actor
	console.log("\n=== Cold actor TX inserts ===");
	for (const n of [1, 10, 100]) {
		const a = fresh();
		const r = await a.benchInsertTransaction(n);
		console.log(`Fresh actor TX x${n}: ${r.elapsedMs.toFixed(1)}ms (${(r.elapsedMs / n).toFixed(3)}ms/op)`);
	}

	// Warm TX: reuse same actor
	console.log("\n=== Warm actor TX inserts ===");
	const a2 = fresh();
	await a2.benchInsertTransaction(10); // warmup
	for (const n of [1, 10, 100]) {
		const r = await a2.benchInsertTransaction(n);
		console.log(`Warm actor TX x${n}: ${r.elapsedMs.toFixed(1)}ms (${(r.elapsedMs / n).toFixed(3)}ms/op)`);
	}

	// Cold batch x1: the 38ms anomaly
	console.log("\n=== Cold actor batch x1 ===");
	for (let i = 0; i < 5; i++) {
		const a = fresh();
		const r = await a.benchInsertBatch(1, 50);
		console.log(`Fresh actor batch x1 attempt ${i}: ${r.elapsedMs.toFixed(1)}ms`);
	}

	process.exit(0);
}

main().catch(e => { console.error(e); process.exit(1); });
