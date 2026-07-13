/**
 * Benchmark data for agentOS marketing copy on rivet.dev.
 *
 * This module mirrors agent-os `website/src/data/bench.ts` (the agentos-sdk.dev
 * site) and must be kept in sync with it: raw constants, formulas, and
 * BENCHMARK_DATE are copied verbatim so both sites cite identical numbers.
 * The source JSON files referenced in the per-constant comments below live in
 * the agent-os repo (scripts/benchmarks/results/).
 *
 * All costs and multipliers are computed from the raw benchmark inputs below.
 * To update, change the input constants and everything recomputes.
 */

// ── Raw benchmark inputs ──

// Cold start (sleep workload, ms)
export const COLDSTART_P50_MS = 4.8;
export const COLDSTART_P95_MS = 5.6;
export const COLDSTART_P99_MS = 6.1;

// Memory per VM (MB, from avgPerVmRssBytes)
// Copy from scripts/benchmarks/results/
export const MEMORY_AGENT_MB = 131;  // memory-pi-session.json  (137207808 / 1024^2)
export const MEMORY_SHELL_MB = 22;   // memory-sleep.json       (23160422 / 1024^2)

// ── Sandbox baselines (external benchmarks) ──

// Date the external sandbox baselines were last verified. Interpolated wherever
// marketing copy cites the baselines so a refresh only touches this constant.
export const BENCHMARK_DATE = 'March 30, 2026';

// Coldstart baseline: E2B (used for cold start comparison only)
export const SANDBOX_COLDSTART_PROVIDER = 'E2B';
export const SANDBOX_COLDSTART_MS = { p50: 440, p95: 950, p99: 3150 };

// Cost/memory baseline: Daytona, cheapest mainstream sandbox provider as of March 30, 2026
// Pricing: $0.0504/vCPU-h + $0.0162/GiB-h. Default minimum: 1 vCPU + 1 GiB RAM.
// Source: https://www.daytona.io/pricing (as of March 30, 2026)
export const SANDBOX_COST_PROVIDER = 'Daytona';
export const SANDBOX_VCPU_COST_PER_HOUR = 0.0504;
export const SANDBOX_GIB_COST_PER_HOUR = 0.0162;
export const SANDBOX_MIN_VCPU = 1;
export const SANDBOX_MIN_MEMORY_GIB = 1;
export const SANDBOX_MIN_MEMORY_MB = SANDBOX_MIN_MEMORY_GIB * 1024;

// ── Self-hosted hardware pricing ──

export const HARDWARE = [
	{ label: 'Hetzner ARM', costPerHour: 3.29 * 1.09 / (30 * 24),    memoryMb: 4096 }, // €3.29/mo
	{ label: 'Hetzner x86', costPerHour: 5.39 * 1.09 / (30 * 24),    memoryMb: 4096 }, // €5.39/mo
	{ label: 'AWS ARM',     costPerHour: 0.0084,                      memoryMb: 1024 },
	{ label: 'AWS x86',     costPerHour: 0.0104,                      memoryMb: 1024 },
] as const;

export const UTILIZATION = 0.7;

// ── Computed data ──

export const sandboxCostPerSec =
	(SANDBOX_MIN_VCPU * SANDBOX_VCPU_COST_PER_HOUR + SANDBOX_MIN_MEMORY_GIB * SANDBOX_GIB_COST_PER_HOUR) / 3600;

function formatCost(perSec: number): string {
	const magnitude = Math.floor(Math.log10(Math.abs(perSec)));
	const decimals = Math.max(0, -magnitude + 1);
	return `$${perSec.toFixed(decimals)}/s`;
}

export interface CostTier {
	label: string;
	value: string;
	multiplier: string;
	ratio: number;
	bar: number;
	/** Concurrent executions packed onto one server at UTILIZATION (the cost denominator). */
	execs: number;
	/** Server RAM in MB for this hardware tier. */
	serverMemMb: number;
	/** Per-execution memory footprint in MB (the workload's memoryMb). */
	workloadMemMb: number;
	/** Raw server price per hour, for "$X/hr server / N execs" copy. */
	costPerHour: number;
}

export interface MemoryData {
	agentOS: string;
	agentOSBar: number;
	sandbox: string;
	sandboxBar: number;
	multiplier: string;
}

export interface WorkloadData {
	label: string;
	description: string;
	memory: MemoryData;
	cost: CostTier[];
	sandboxCost: string;
}

export function computeWorkload(label: string, description: string, memoryMb: number): WorkloadData {
	const memBar = Math.round((memoryMb / SANDBOX_MIN_MEMORY_MB) * 1000) / 10;
	const memMultiplier = Math.round(SANDBOX_MIN_MEMORY_MB / memoryMb);

	const cost: CostTier[] = HARDWARE.map((hw) => {
		const instanceCostPerSec = hw.costPerHour / 3600;
		const effectiveExecs = Math.floor(Math.floor(hw.memoryMb / memoryMb) * UTILIZATION);
		const costPerExecSec = instanceCostPerSec / effectiveExecs;
		const ratio = sandboxCostPerSec / costPerExecSec;
		const bar = Math.round((costPerExecSec / sandboxCostPerSec) * 1000) / 10;
		return {
			label: hw.label,
			value: formatCost(costPerExecSec),
			multiplier: `${Math.round(ratio)}x cheaper`,
			ratio: Math.round(ratio),
			bar,
			execs: effectiveExecs,
			serverMemMb: hw.memoryMb,
			workloadMemMb: memoryMb,
			costPerHour: hw.costPerHour,
		};
	});

	return {
		label,
		description,
		memory: {
			agentOS: `~${memoryMb} MB`,
			agentOSBar: memBar,
			sandbox: `~${SANDBOX_MIN_MEMORY_MB} MB`,
			sandboxBar: 100,
			multiplier: `${memMultiplier}x smaller`,
		},
		cost,
		sandboxCost: formatCost(sandboxCostPerSec),
	};
}

// Pre-computed workloads
export const benchColdStart = [
	{ label: 'p50' as const, agentOS: COLDSTART_P50_MS, sandbox: SANDBOX_COLDSTART_MS.p50 },
	{ label: 'p95' as const, agentOS: COLDSTART_P95_MS, sandbox: SANDBOX_COLDSTART_MS.p95 },
	{ label: 'p99' as const, agentOS: COLDSTART_P99_MS, sandbox: SANDBOX_COLDSTART_MS.p99 },
];

export const benchWorkloads = {
	agent: computeWorkload(
		'Coding agent',
		'Pi coding agent session with MCP servers and mounted file systems',
		MEMORY_AGENT_MB,
	),
	shell: computeWorkload(
		'Execution',
		'Minimal shell workload running simple commands',
		MEMORY_SHELL_MB,
	),
};

export type WorkloadKey = keyof typeof benchWorkloads;
