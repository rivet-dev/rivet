#!/usr/bin/env -S tsx

/**
 * SQLite Benchmark Script
 *
 * Compares batch vs non-batch performance:
 * 1. Filesystem + Native SQLite (better-sqlite3)
 * 2. Filesystem + KV Filesystem (wa-sqlite with file-backed KV)
 */

import Table from "cli-table3";
import { createFileSystemDriver } from "@/drivers/file-system/mod";
import { registry } from "../fixtures/driver-test-suite/registry";

interface TimingResult {
	batch: number;
	nonBatch: number;
}

interface BenchmarkResult {
	name: string;
	insert: TimingResult;
	select: TimingResult;
	update: TimingResult;
}

const ROW_COUNT = 100;
const QUERY_COUNT = 100;

type Client = Awaited<ReturnType<typeof registry.start>>["client"];

async function runBenchmark(client: Client, name: string): Promise<BenchmarkResult> {
	const results: BenchmarkResult = {
		name,
		insert: { batch: 0, nonBatch: 0 },
		select: { batch: 0, nonBatch: 0 },
		update: { batch: 0, nonBatch: 0 },
	};

	// --- INSERT ---
	// Non-batch
	{
		const handle = await client.dbActorRaw.getOrCreate([`bench-nonbatch-${Date.now()}`]);
		const start = performance.now();
		for (let i = 0; i < ROW_COUNT; i++) {
			await handle.insertValue(`User ${i}`);
		}
		results.insert.nonBatch = performance.now() - start;
	}

	// Batch
	{
		const handle = await client.dbActorRaw.getOrCreate([`bench-batch-${Date.now()}`]);
		const start = performance.now();
		await handle.bulkInsert(ROW_COUNT);
		results.insert.batch = performance.now() - start;
	}

	// --- SELECT ---
	const handle = await client.dbActorRaw.getOrCreate([`bench-select-${Date.now()}`]);
	await handle.bulkInsert(ROW_COUNT);

	// Batch (single query)
	{
		const start = performance.now();
		await handle.getValues();
		results.select.batch = performance.now() - start;
	}

	// Non-batch (100 queries)
	{
		const start = performance.now();
		for (let i = 0; i < QUERY_COUNT; i++) {
			await handle.getCount();
		}
		results.select.nonBatch = performance.now() - start;
	}

	// --- UPDATE ---
	// Non-batch
	{
		const start = performance.now();
		for (let i = 1; i <= QUERY_COUNT; i++) {
			await handle.updateValue(i, `Updated ${i}`);
		}
		results.update.nonBatch = performance.now() - start;
	}

	// Batch
	{
		const start = performance.now();
		await handle.bulkUpdate(QUERY_COUNT);
		results.update.batch = performance.now() - start;
	}

	return results;
}

function ms(n: number): string {
	return n === 0 ? "-" : `${n.toFixed(2)}ms`;
}

function perOp(total: number, count: number): string {
	return total === 0 ? "-" : `${(total / count).toFixed(3)}ms`;
}

function speedup(nonBatch: number, batch: number): string {
	if (nonBatch === 0 || batch === 0) return "-";
	return `${(nonBatch / batch).toFixed(1)}x`;
}

function printResults(results: BenchmarkResult[]): void {
	console.log(`\nBenchmark: ${ROW_COUNT} rows, ${QUERY_COUNT} queries\n`);

	// INSERT table
	const insertTable = new Table({
		head: ["Driver", "Batch", "Per-Op", "Non-Batch", "Per-Op", "Speedup"],
	});
	for (const r of results) {
		insertTable.push([
			r.name,
			ms(r.insert.batch),
			perOp(r.insert.batch, ROW_COUNT),
			ms(r.insert.nonBatch),
			perOp(r.insert.nonBatch, ROW_COUNT),
			speedup(r.insert.nonBatch, r.insert.batch),
		]);
	}
	console.log("INSERT");
	console.log(insertTable.toString());

	// SELECT table
	const selectTable = new Table({
		head: ["Driver", "Batch", "Non-Batch", "Per-Query", "Speedup"],
	});
	for (const r of results) {
		selectTable.push([
			r.name,
			ms(r.select.batch),
			ms(r.select.nonBatch),
			perOp(r.select.nonBatch, QUERY_COUNT),
			speedup(r.select.nonBatch, r.select.batch),
		]);
	}
	console.log("\nSELECT");
	console.log(selectTable.toString());

	// UPDATE table
	const updateTable = new Table({
		head: ["Driver", "Batch", "Per-Op", "Non-Batch", "Per-Op", "Speedup"],
	});
	for (const r of results) {
		updateTable.push([
			r.name,
			ms(r.update.batch),
			perOp(r.update.batch, QUERY_COUNT),
			ms(r.update.nonBatch),
			perOp(r.update.nonBatch, QUERY_COUNT),
			speedup(r.update.nonBatch, r.update.batch),
		]);
	}
	console.log("\nUPDATE");
	console.log(updateTable.toString());

	// Cross-driver comparison
	const baseline = results[0];
	if (baseline && results.length > 1) {
		const compTable = new Table({
			head: ["Driver", "Insert (batch)", "vs Baseline", "Select (batch)", "vs Baseline"],
		});
		for (const r of results) {
			compTable.push([
				r.name,
				ms(r.insert.batch),
				`${(r.insert.batch / baseline.insert.batch).toFixed(1)}x`,
				ms(r.select.batch),
				`${(r.select.batch / baseline.select.batch).toFixed(1)}x`,
			]);
		}
		console.log("\nCROSS-DRIVER (batch mode)");
		console.log(compTable.toString());
	}
}

async function main(): Promise<void> {
	console.log("SQLite Benchmark\n");

	const results: BenchmarkResult[] = [];

	// 1. Native SQLite
	console.log("1. Native SQLite...");
	try {
		const { client } = await registry.start({
			driver: createFileSystemDriver({ useNativeSqlite: true }),
			defaultServerPort: 6430,
		});
		results.push(await runBenchmark(client, "Native SQLite"));
		console.log("  Done");
	} catch (err) {
		console.log(`  Skipped: ${err}`);
	}

	// 2. KV Filesystem
	console.log("2. KV Filesystem...");
	try {
		const { client } = await registry.start({
			driver: createFileSystemDriver({ useNativeSqlite: false }),
			defaultServerPort: 6431,
		});
		results.push(await runBenchmark(client, "KV Filesystem"));
		console.log("  Done");
	} catch (err) {
		console.log(`  Skipped: ${err}`);
	}

	// 3. Engine (connects to running engine on 6420)
	console.log("3. Engine (localhost:6420)...");
	try {
		const { client } = await registry.start({
			endpoint: "http://localhost:6420",
		});
		results.push(await runBenchmark(client, "Engine"));
		console.log("  Done");
	} catch (err) {
		console.log(`  Skipped: ${err}`);
	}

	printResults(results);
}

main().catch(console.error);
