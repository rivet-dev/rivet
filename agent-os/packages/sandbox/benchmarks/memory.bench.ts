/**
 * Memory overhead benchmark for AgentOS WASM VMs.
 *
 * Measures incremental RSS and heap per live VM instance by spinning up
 * N VMs each running a workload, sampling memory, then tearing them down.
 *
 * Workloads:
 *   --workload=sleep  (default) Minimal VM with coreutils, running `sleep 99999`
 *   --workload=pi     Full VM with common + PI agent in RPC mode
 *
 * Pass --batch=N for a single batch size (recommended for clean RSS).
 * Without --batch, runs default batch sizes sequentially.
 *
 * Usage:
 *   npx tsx --expose-gc benchmarks/memory.bench.ts
 *   npx tsx --expose-gc benchmarks/memory.bench.ts --workload=pi --batch=5
 */

import type { AgentOs } from "@rivet-dev/agent-os-core";
import {
	MAX_CONCURRENCY,
	MEMORY_ITERATIONS,
	WORKLOADS,
	type Workload,
	forceGC,
	formatBytes,
	getHardware,
	printTable,
	round,
	sleep,
} from "./bench-utils.js";

// Default batch sizes when not using --batch.
const DEFAULT_BATCH_SIZES = [1, 10, 50, 100];

interface MemoryEntry {
	batchSize: number;
	iterations: number;
	workload: string;
	/** First iteration (cold module cache). */
	coldPerVmRssBytes: number;
	coldPerVmHeapBytes: number;
	/** Average of subsequent iterations (warm module cache). */
	warmPerVmRssBytes: number;
	warmPerVmHeapBytes: number;
	/** Average across all iterations. */
	totalDeltaRssBytes: number;
	totalDeltaHeapBytes: number;
	perVmRssBytes: number;
	perVmHeapBytes: number;
	teardownReclaimedRssBytes: number;
}

async function measureBatch(
	workload: Workload,
	batchSize: number,
): Promise<MemoryEntry> {
	const rssSamples: number[] = [];
	const heapSamples: number[] = [];
	const reclaimSamples: number[] = [];

	for (let iter = 0; iter < MEMORY_ITERATIONS; iter++) {
		// Baseline. Multiple GC passes to flush incremental/concurrent phases.
		forceGC();
		forceGC();
		await sleep(100);
		const baseline = process.memoryUsage();

		// Create VMs and start the workload in each.
		const vms: AgentOs[] = [];
		let remaining = batchSize;

		while (remaining > 0) {
			const chunk = Math.min(remaining, MAX_CONCURRENCY);
			const batch = await Promise.all(
				Array.from({ length: chunk }, async () => {
					const vm = await workload.createVm();
					workload.start(vm);
					return vm;
				}),
			);
			vms.push(...batch);
			remaining -= chunk;
		}

		// Let processes fully initialize.
		await sleep(workload.settleMs);

		// Measure after init.
		forceGC();
		forceGC();
		await sleep(100);
		const afterInit = process.memoryUsage();

		const rssDelta = afterInit.rss - baseline.rss;
		const heapDelta = afterInit.heapUsed - baseline.heapUsed;

		console.error(
			`    iter ${iter}: rss_delta=${formatBytes(rssDelta)} heap_delta=${formatBytes(heapDelta)}`,
		);
		rssSamples.push(rssDelta);
		heapSamples.push(heapDelta);

		// Teardown.
		await Promise.all(vms.map((vm) => vm.dispose()));
		forceGC();
		forceGC();
		await sleep(100);
		const afterTeardown = process.memoryUsage();

		reclaimSamples.push(afterInit.rss - afterTeardown.rss);
	}

	// Split cold (first iteration) vs warm (subsequent iterations).
	const coldRss = rssSamples[0];
	const coldHeap = heapSamples[0];
	const warmRssSamples = rssSamples.slice(1);
	const warmHeapSamples = heapSamples.slice(1);
	const warmRss =
		warmRssSamples.length > 0
			? warmRssSamples.reduce((a, b) => a + b, 0) / warmRssSamples.length
			: coldRss;
	const warmHeap =
		warmHeapSamples.length > 0
			? warmHeapSamples.reduce((a, b) => a + b, 0) / warmHeapSamples.length
			: coldHeap;

	// Average across all iterations.
	const avgRss = rssSamples.reduce((a, b) => a + b, 0) / MEMORY_ITERATIONS;
	const avgHeap = heapSamples.reduce((a, b) => a + b, 0) / MEMORY_ITERATIONS;
	const avgReclaim =
		reclaimSamples.reduce((a, b) => a + b, 0) / MEMORY_ITERATIONS;

	return {
		batchSize,
		iterations: MEMORY_ITERATIONS,
		workload: workload.name,
		coldPerVmRssBytes: Math.round(coldRss / batchSize),
		coldPerVmHeapBytes: Math.round(coldHeap / batchSize),
		warmPerVmRssBytes: Math.round(warmRss / batchSize),
		warmPerVmHeapBytes: Math.round(warmHeap / batchSize),
		totalDeltaRssBytes: Math.round(avgRss),
		totalDeltaHeapBytes: Math.round(avgHeap),
		perVmRssBytes: Math.round(avgRss / batchSize),
		perVmHeapBytes: Math.round(avgHeap / batchSize),
		teardownReclaimedRssBytes: Math.round(avgReclaim),
	};
}

function parseArgs(): { batchSizes: number[]; workload: Workload } {
	const batchArg = process.argv.find((a) => a.startsWith("--batch="));
	const workloadArg = process.argv.find((a) => a.startsWith("--workload="));

	let batchSizes = DEFAULT_BATCH_SIZES;
	if (batchArg) {
		const val = parseInt(batchArg.split("=")[1], 10);
		if (isNaN(val) || val < 1) {
			console.error(`Invalid --batch value: ${batchArg}`);
			process.exit(1);
		}
		batchSizes = [val];
	}

	let workload = WORKLOADS.sleep;
	if (workloadArg) {
		const name = workloadArg.split("=")[1];
		if (!WORKLOADS[name]) {
			console.error(
				`Unknown workload: ${name}. Available: ${Object.keys(WORKLOADS).join(", ")}`,
			);
			process.exit(1);
		}
		workload = WORKLOADS[name];
	}

	return { batchSizes, workload };
}

async function main() {
	if (!global.gc) {
		console.error(
			"ERROR: Run with --expose-gc flag\n" +
				"  npx tsx --expose-gc benchmarks/memory.bench.ts",
		);
		process.exit(1);
	}

	const { batchSizes, workload } = parseArgs();
	const hardware = getHardware();
	console.error(`=== Memory Overhead Benchmark ===`);
	console.error(`Workload: ${workload.name} — ${workload.description}`);
	console.error(`CPU: ${hardware.cpu}`);
	console.error(`RAM: ${hardware.ram} | Node: ${hardware.node}`);
	console.error(`Iterations per batch: ${MEMORY_ITERATIONS}`);
	console.error(`Batch sizes: ${batchSizes.join(", ")}`);

	const results: MemoryEntry[] = [];

	for (const batchSize of batchSizes) {
		console.error(`\n--- batch=${batchSize} ---`);
		const entry = await measureBatch(workload, batchSize);
		results.push(entry);
		console.error(
			`  cold per-VM RSS: ${formatBytes(entry.coldPerVmRssBytes)} | heap: ${formatBytes(entry.coldPerVmHeapBytes)}`,
		);
		console.error(
			`  warm per-VM RSS: ${formatBytes(entry.warmPerVmRssBytes)} | heap: ${formatBytes(entry.warmPerVmHeapBytes)}`,
		);
		console.error(
			`  teardown reclaimed: ${formatBytes(entry.teardownReclaimedRssBytes)}`,
		);
	}

	// Summary table.
	printTable(
		[
			"workload",
			"batch",
			"cold RSS/VM",
			"cold heap/VM",
			"warm RSS/VM",
			"warm heap/VM",
			"reclaimed",
		],
		results.map((r) => [
			r.workload,
			r.batchSize,
			formatBytes(r.coldPerVmRssBytes),
			formatBytes(r.coldPerVmHeapBytes),
			formatBytes(r.warmPerVmRssBytes),
			formatBytes(r.warmPerVmHeapBytes),
			formatBytes(r.teardownReclaimedRssBytes),
		]),
	);

	// JSON to stdout.
	console.log(JSON.stringify({ hardware, workload: workload.name, results }, null, 2));
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
