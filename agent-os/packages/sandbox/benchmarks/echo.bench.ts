/**
 * Cold-start and warm-start latency benchmark for WASM shell echo.
 *
 * Measures:
 *   - Cold start: time to create an AgentOS VM + complete first exec("echo hello")
 *   - Warm start: time for a second exec("echo hello") on an already-initialized VM
 *   - Both sequential and concurrent modes at various batch sizes
 *
 * The WASM coreutils package provides the shell and echo command compiled to
 * WebAssembly. Each cold-start measurement includes kernel creation, WASM
 * runtime mounting, and first command execution.
 *
 * Usage: npx tsx benchmarks/echo.bench.ts
 */

import type { AgentOs } from "@rivet-dev/agent-os-core";
import {
	BATCH_SIZES,
	ECHO_COMMAND,
	EXPECTED_OUTPUT,
	ITERATIONS,
	MAX_CONCURRENCY,
	WARMUP_ITERATIONS,
	createBenchVm,
	getHardware,
	printTable,
	round,
	stats,
} from "./bench-utils.js";

interface ColdStartEntry {
	batchSize: number;
	mode: "sequential" | "concurrent";
	iterations: number;
	coldStart: ReturnType<typeof stats>;
	warmStart: ReturnType<typeof stats>;
}

async function measureOne(): Promise<{ coldMs: number; warmMs: number }> {
	const t0 = performance.now();
	const vm = await createBenchVm();
	const result1 = await vm.exec(ECHO_COMMAND);
	const coldMs = performance.now() - t0;

	if (result1.stdout !== EXPECTED_OUTPUT) {
		throw new Error(
			`Unexpected cold-start output: ${JSON.stringify(result1.stdout)} (expected ${JSON.stringify(EXPECTED_OUTPUT)})`,
		);
	}

	const t1 = performance.now();
	const result2 = await vm.exec(ECHO_COMMAND);
	const warmMs = performance.now() - t1;

	if (result2.stdout !== EXPECTED_OUTPUT) {
		throw new Error(
			`Unexpected warm-start output: ${JSON.stringify(result2.stdout)} (expected ${JSON.stringify(EXPECTED_OUTPUT)})`,
		);
	}

	await vm.dispose();
	return { coldMs, warmMs };
}

async function benchSequential(batchSize: number): Promise<ColdStartEntry> {
	const coldSamples: number[] = [];
	const warmSamples: number[] = [];

	for (let iter = 0; iter < WARMUP_ITERATIONS + ITERATIONS; iter++) {
		const iterCold: number[] = [];
		const iterWarm: number[] = [];

		for (let i = 0; i < batchSize; i++) {
			const { coldMs, warmMs } = await measureOne();
			iterCold.push(coldMs);
			iterWarm.push(warmMs);
		}

		// Skip warmup iterations.
		if (iter >= WARMUP_ITERATIONS) {
			coldSamples.push(...iterCold);
			warmSamples.push(...iterWarm);
		}
	}

	return {
		batchSize,
		mode: "sequential",
		iterations: ITERATIONS,
		coldStart: stats(coldSamples),
		warmStart: stats(warmSamples),
	};
}

async function benchConcurrent(batchSize: number): Promise<ColdStartEntry> {
	const effectiveConcurrency = Math.min(batchSize, MAX_CONCURRENCY);
	const coldSamples: number[] = [];
	const warmSamples: number[] = [];

	for (let iter = 0; iter < WARMUP_ITERATIONS + ITERATIONS; iter++) {
		const iterCold: number[] = [];
		const iterWarm: number[] = [];
		let remaining = batchSize;

		while (remaining > 0) {
			const chunk = Math.min(remaining, effectiveConcurrency);
			const results = await Promise.all(
				Array.from({ length: chunk }, () => measureOne()),
			);
			for (const { coldMs, warmMs } of results) {
				iterCold.push(coldMs);
				iterWarm.push(warmMs);
			}
			remaining -= chunk;
		}

		if (iter >= WARMUP_ITERATIONS) {
			coldSamples.push(...iterCold);
			warmSamples.push(...iterWarm);
		}
	}

	return {
		batchSize,
		mode: "concurrent",
		iterations: ITERATIONS,
		coldStart: stats(coldSamples),
		warmStart: stats(warmSamples),
	};
}

async function main() {
	const hardware = getHardware();
	console.error(`=== WASM Echo Benchmark ===`);
	console.error(`CPU: ${hardware.cpu}`);
	console.error(
		`Cores: ${hardware.cores} | Max concurrency: ${MAX_CONCURRENCY}`,
	);
	console.error(`RAM: ${hardware.ram} | Node: ${hardware.node}`);
	console.error(`Iterations: ${ITERATIONS} (+ ${WARMUP_ITERATIONS} warmup)`);
	console.error(`Batch sizes: ${BATCH_SIZES.join(", ")}`);
	console.error(`Command: ${ECHO_COMMAND}`);

	const results: ColdStartEntry[] = [];

	for (const batchSize of BATCH_SIZES) {
		console.error(`\n--- batch=${batchSize}, mode=sequential ---`);
		const seq = await benchSequential(batchSize);
		results.push(seq);
		console.error(
			`  cold: mean=${seq.coldStart.mean}ms p50=${seq.coldStart.p50}ms p95=${seq.coldStart.p95}ms`,
		);
		console.error(
			`  warm: mean=${seq.warmStart.mean}ms p50=${seq.warmStart.p50}ms p95=${seq.warmStart.p95}ms`,
		);

		console.error(`\n--- batch=${batchSize}, mode=concurrent ---`);
		const conc = await benchConcurrent(batchSize);
		results.push(conc);
		console.error(
			`  cold: mean=${conc.coldStart.mean}ms p50=${conc.coldStart.p50}ms p95=${conc.coldStart.p95}ms`,
		);
		console.error(
			`  warm: mean=${conc.warmStart.mean}ms p50=${conc.warmStart.p50}ms p95=${conc.warmStart.p95}ms`,
		);
	}

	// Summary table
	printTable(
		[
			"batch",
			"mode",
			"cold mean",
			"cold p50",
			"cold p95",
			"warm mean",
			"warm p50",
			"warm p95",
		],
		results.map((r) => [
			r.batchSize,
			r.mode,
			`${r.coldStart.mean}ms`,
			`${r.coldStart.p50}ms`,
			`${r.coldStart.p95}ms`,
			`${r.warmStart.mean}ms`,
			`${r.warmStart.p50}ms`,
			`${r.warmStart.p95}ms`,
		]),
	);

	// JSON to stdout
	console.log(JSON.stringify({ hardware, results }, null, 2));
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
