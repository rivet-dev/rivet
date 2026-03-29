import { AgentOs } from "@rivet-dev/agent-os-core";
import common, { coreutils } from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import os from "node:os";

// Benchmark parameters. Keep batch sizes minimal for fast iteration.
export const BATCH_SIZES = [1, 10];
export const ITERATIONS = 5;
export const WARMUP_ITERATIONS = 1;
export const MAX_CONCURRENCY = Math.max(1, os.availableParallelism() - 4);

export const MEMORY_ITERATIONS = 3;
export const ECHO_COMMAND = "echo hello";
export const EXPECTED_OUTPUT = "hello\n";

// ── Workload abstraction ────────────────────────────────────────────

/** A workload describes how to create a VM and start a long-running process for memory measurement. */
export interface Workload {
	name: string;
	description: string;
	createVm: () => Promise<AgentOs>;
	/** Start a long-running process so the Worker thread stays alive. */
	start: (vm: AgentOs) => void;
	/** Time to wait after start for the process to fully initialize. */
	settleMs: number;
}

export const WORKLOADS: Record<string, Workload> = {
	sleep: {
		name: "sleep",
		description: "Minimal VM with coreutils, running `sleep 99999`",
		createVm: () => AgentOs.create({ software: [coreutils] }),
		start: (vm) => {
			vm.spawn("sleep", ["99999"]);
		},
		settleMs: 500,
	},
	"pi-sdk": {
		name: "pi-sdk",
		description:
			"VM with common + full PI SDK loaded via dynamic import",
		createVm: () => AgentOs.create({ software: [common, pi] }),
		start: (vm) => {
			vm.spawn(
				"node",
				[
					"-e",
					'import("/root/node_modules/@mariozechner/pi-coding-agent/dist/index.js").then(() => console.log("PI SDK loaded")).catch(e => console.error("PI SDK failed:", e.message)); setTimeout(() => {}, 999999);',
				],
				{
					streamStdin: true,
					env: { ANTHROPIC_API_KEY: "bench-key" },
				},
			);
		},
		settleMs: 5000,
	},
};

// ── VM creation helpers ─────────────────────────────────────────────

/**
 * Create a fresh AgentOS VM with only coreutils (WASM shell + echo).
 * This is the minimal setup needed to run shell commands.
 */
export async function createBenchVm(): Promise<AgentOs> {
	return AgentOs.create({
		software: [coreutils],
	});
}

// ── Stats and formatting ────────────────────────────────────────────

export function percentile(sorted: number[], p: number): number {
	const idx = Math.ceil((p / 100) * sorted.length) - 1;
	return sorted[Math.max(0, idx)];
}

export function stats(samples: number[]) {
	const sorted = [...samples].sort((a, b) => a - b);
	const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
	return {
		mean: round(mean),
		p50: round(percentile(sorted, 50)),
		p95: round(percentile(sorted, 95)),
		p99: round(percentile(sorted, 99)),
		min: round(sorted[0]),
		max: round(sorted[sorted.length - 1]),
	};
}

export function round(n: number, decimals = 2): number {
	const f = 10 ** decimals;
	return Math.round(n * f) / f;
}

export function getHardware() {
	const cpus = os.cpus();
	return {
		cpu: cpus[0]?.model ?? "unknown",
		cores: os.availableParallelism(),
		ram: `${round(os.totalmem() / 1024 ** 3, 1)} GB`,
		node: process.version,
		os: `${os.type()} ${os.release()}`,
		arch: os.arch(),
	};
}

export function forceGC() {
	if (global.gc) {
		global.gc();
	} else {
		console.error("WARNING: global.gc not available. Run with --expose-gc");
	}
}

export async function sleep(ms: number): Promise<void> {
	return new Promise((r) => setTimeout(r, ms));
}

export function formatBytes(bytes: number): string {
	if (Math.abs(bytes) < 1024) return `${bytes} B`;
	const mb = bytes / (1024 * 1024);
	return `${round(mb, 2)} MB`;
}

/** Print a table to stderr for human readability. */
export function printTable(
	headers: string[],
	rows: (string | number)[][],
): void {
	const widths = headers.map((h, i) =>
		Math.max(h.length, ...rows.map((r) => String(r[i]).length)),
	);
	const sep = widths.map((w) => "-".repeat(w)).join(" | ");
	const fmt = (row: (string | number)[]) =>
		row.map((c, i) => String(c).padStart(widths[i])).join(" | ");

	console.error("");
	console.error(fmt(headers));
	console.error(sep);
	for (const row of rows) {
		console.error(fmt(row));
	}
	console.error("");
}
