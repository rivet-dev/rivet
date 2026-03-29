/**
 * Benchmark data for agentOS marketing pages.
 *
 * All costs and multipliers are computed from the raw benchmark inputs below.
 * To update, change the input constants and everything recomputes.
 *
 * Source JSON files — copy from ~/a1/scripts/benchmarks/results/:
 *   coldstart-sleep.json     → COLDSTART_P50/P95/P99_MS  (result.p50Ms, p95Ms, p99Ms)
 *   memory-sleep.json        → MEMORY_SHELL_MB           (result.avgPerVmRssBytes / 1024^2, rounded)
 *   memory-pi-session.json   → MEMORY_AGENT_MB           (result.avgPerVmRssBytes / 1024^2, rounded)
 */

// ── Raw benchmark inputs ──

// Cold start (sleep workload, ms)
export const COLDSTART_P50_MS = 4.8;
export const COLDSTART_P95_MS = 5.6;
export const COLDSTART_P99_MS = 6.1;

// Memory per VM (MB, from avgPerVmRssBytes)
// Copy from ~/a1/scripts/benchmarks/results/
export const MEMORY_AGENT_MB = 131;  // memory-pi-session.json  (137207808 / 1024^2)
export const MEMORY_SHELL_MB = 22;   // memory-sleep.json       (23160422 / 1024^2)

// ── Sandbox baselines (external benchmarks) ──

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
	{ label: 'AWS ARM',     costPerHour: 0.0084,                      memoryMb: 1024 },
	{ label: 'AWS x86',     costPerHour: 0.0104,                      memoryMb: 1024 },
	{ label: 'Hetzner ARM', costPerHour: 3.29 * 1.09 / (30 * 24),    memoryMb: 4096 }, // €3.29/mo
	{ label: 'Hetzner x86', costPerHour: 5.39 * 1.09 / (30 * 24),    memoryMb: 4096 }, // €5.39/mo
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
		'Full coding agent',
		'Pi coding agent session with MCP servers and mounted file systems',
		MEMORY_AGENT_MB,
	),
	shell: computeWorkload(
		'Simple shell command',
		'Minimal shell workload running simple commands',
		MEMORY_SHELL_MB,
	),
};

export type WorkloadKey = keyof typeof benchWorkloads;
