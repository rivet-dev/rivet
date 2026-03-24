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

	// TX x1 cold vs warm
	console.log("=== TX x1 cold vs warm ===");
	const a1 = fresh();
	const r1 = await a1.benchInsertTransaction(1);
	console.log(`TX x1 cold:  ${r1.elapsedMs.toFixed(1)}ms`);
	const r2 = await a1.benchInsertTransaction(1);
	console.log(`TX x1 warm:  ${r2.elapsedMs.toFixed(1)}ms`);
	const r3 = await a1.benchInsertTransaction(1);
	console.log(`TX x1 warm2: ${r3.elapsedMs.toFixed(1)}ms`);

	// TX x10 cold vs warm
	console.log("\n=== TX x10 cold vs warm ===");
	const a1b = fresh();
	const r1b = await a1b.benchInsertTransaction(10);
	console.log(`TX x10 cold: ${r1b.elapsedMs.toFixed(1)}ms`);
	const r2b = await a1b.benchInsertTransaction(10);
	console.log(`TX x10 warm: ${r2b.elapsedMs.toFixed(1)}ms`);

	// Point reads: seed, then multiple rounds
	console.log("\n=== Point reads ===");
	const a2 = fresh();
	await a2.benchInsertTransaction(1000);
	console.log("Seeded 1000 rows");
	for (let i = 0; i < 5; i++) {
		const r = await a2.benchPointRead(100);
		console.log(`Point read x100 round ${i}: ${r.elapsedMs.toFixed(1)}ms (${(r.elapsedMs / 100).toFixed(3)}ms/op)`);
	}
	for (let i = 0; i < 3; i++) {
		const r = await a2.benchPointRead(1000);
		console.log(`Point read x1000 round ${i}: ${r.elapsedMs.toFixed(1)}ms (${(r.elapsedMs / 1000).toFixed(3)}ms/op)`);
	}

	// Large payload: insert then read multiple times
	console.log("\n=== Large payload 4KB ===");
	const a3 = fresh();
	const rL1 = await a3.benchLargePayload(100, 4096);
	console.log(`Insert 4KB x100: ${rL1.insertElapsedMs.toFixed(1)}ms`);
	console.log(`Read 4KB x100:   ${rL1.readElapsedMs.toFixed(1)}ms`);
	// Read again by querying directly
	const rL2 = await a3.benchPointRead(100);
	console.log(`Point read after large payload: ${rL2.elapsedMs.toFixed(1)}ms`);

	// Batch x1 cold vs warm
	console.log("\n=== Batch x1 cold vs warm ===");
	const a4 = fresh();
	const rB1 = await a4.benchInsertBatch(1, 50);
	console.log(`Batch x1 cold: ${rB1.elapsedMs.toFixed(1)}ms`);
	const rB2 = await a4.benchInsertBatch(1, 50);
	console.log(`Batch x1 warm: ${rB2.elapsedMs.toFixed(1)}ms`);
	const rB3 = await a4.benchInsertBatch(1, 50);
	console.log(`Batch x1 warm2: ${rB3.elapsedMs.toFixed(1)}ms`);

	// JSON insert cold vs warm
	console.log("\n=== JSON insert ===");
	const a5 = fresh();
	const rJ1 = await a5.benchJson(10);
	console.log(`JSON x10 cold: insert=${rJ1.insertElapsedMs.toFixed(1)}ms extract=${rJ1.jsonExtract.elapsedMs.toFixed(1)}ms each=${rJ1.jsonEach.elapsedMs.toFixed(1)}ms`);
	const rJ2 = await a5.benchJson(10);
	console.log(`JSON x10 warm: insert=${rJ2.insertElapsedMs.toFixed(1)}ms extract=${rJ2.jsonExtract.elapsedMs.toFixed(1)}ms each=${rJ2.jsonEach.elapsedMs.toFixed(1)}ms`);

	process.exit(0);
}

main().catch(e => { console.error(e); process.exit(1); });
