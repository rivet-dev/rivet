import type { Rivet } from "@rivet-gg/cloud";

/** Human-readable plan names. `pro` is presented as "Hobby". */
export const PLAN_LABELS: Record<string, string> = {
	free: "Free",
	pro: "Hobby",
	team: "Team",
	enterprise: "Enterprise",
	byoc: "BYOC",
};

/**
 * Rivet Compute pricing. Compute is billed per active second based on each
 * actor's configured CPU and memory.
 *
 * cost = active_seconds × (vcpus × cpu_per_vcpu_second + memory_gib × memory_per_gib_second)
 *
 * One vCPU is half a physical core. The Free plan is limited to 1 vCPU; paid
 * plans allow up to 8 vCPU.
 */
export const COMPUTE = {
	cpuPerVcpuSecond: 0.000033,
	memoryPerGibSecond: 0.0000029,
	maxVcpu: 8,
	freeMaxVcpu: 1,
};

/**
 * Monthly compute spend cap in USD per plan. `null` means no cap (and no
 * progress bar). Only the free plan is capped at $5/month of compute; paid
 * plans have no compute cap.
 */
export const COMPUTE_MONTHLY_CAP_USD: Record<string, number | null> = {
	free: 5,
	pro: null,
	team: null,
	enterprise: null,
};

/**
 * Compute dollars to add on top of the usage endpoint's `totalCents`. Paid
 * plans already include compute there; only free's capped compute is left out.
 */
export function computeOutsideTotalUsd(
	plan: string,
	computeDollars: number,
): number {
	const cap = COMPUTE_MONTHLY_CAP_USD[plan] ?? null;
	return cap != null ? Math.min(computeDollars, cap) : 0;
}

/** Compute cost in dollars per active second for the given actor config. */
export function computeCostPerSecond(vcpus: number, memoryMb: number): number {
	return (
		vcpus * COMPUTE.cpuPerVcpuSecond +
		(memoryMb / 1024) * COMPUTE.memoryPerGibSecond
	);
}

export type PlanRow = { label: string; value?: string };

/**
 * Plan catalog. `rows` mirror the pricing page on rivet.dev: label/value
 * specs for cloud plans, label-only rows for Enterprise.
 */
export const PLANS = [
	{
		id: "free",
		price: "$0",
		description: "For prototyping and small projects.",
		rows: [
			{ label: "Awake Actor Hours", value: "100,000 /mo max" },
			{ label: "Compute", value: "$5 /mo max" },
			{ label: "Max vCPU", value: "1" },
			{ label: "Storage", value: "5GB max" },
			{ label: "Reads", value: "200M /mo max" },
			{ label: "Writes", value: "5M /mo max" },
			{ label: "Egress", value: "100GB max" },
			{ label: "Support", value: "Community" },
		],
	},
	{
		id: "pro",
		price: "$20",
		usageBased: true,
		description: "For scaling applications.",
		rows: [
			{ label: "Awake Actor Hours", value: "400,000 /mo" },
			{ label: "Compute", value: "Usage-based" },
			{ label: "Max vCPU", value: "8" },
			{ label: "Storage", value: "5GB" },
			{ label: "Reads", value: "25B /mo" },
			{ label: "Writes", value: "50M /mo" },
			{ label: "Egress", value: "1TB" },
			{ label: "Support", value: "Email" },
		],
	},
	{
		id: "team",
		price: "$200",
		usageBased: true,
		description: "For growing teams and businesses.",
		rows: [
			{ label: "Awake Actor Hours", value: "400,000 /mo" },
			{ label: "Compute", value: "Usage-based" },
			{ label: "Max vCPU", value: "8" },
			{ label: "Storage", value: "5GB" },
			{ label: "Reads", value: "25B /mo" },
			{ label: "Writes", value: "50M /mo" },
			{ label: "Egress", value: "1TB" },
			{ label: "Support", value: "Slack & Email" },
		],
	},
	{
		id: "enterprise",
		price: "Custom",
		custom: true,
		description:
			"For organizations with compliance and support requirements.",
		rows: [
			{ label: "Everything in Team" },
			{ label: "Priority Support" },
			{ label: "SLA" },
			{ label: "OIDC SSO provider" },
			{ label: "Audit logs" },
			{ label: "Custom Roles" },
			{ label: "Device Tracking" },
			{ label: "Volume Pricing" },
		],
	},
] as const satisfies readonly {
	id: string;
	price: string;
	description: string;
	rows: readonly PlanRow[];
	usageBased?: boolean;
	custom?: boolean;
}[];

export type PlanId = (typeof PLANS)[number]["id"];

/** Catalog entry for an API plan id, falling back to Free for unknown ids. */
export const findPlan = (id: string | undefined) =>
	PLANS.find((entry) => entry.id === id) ?? getPlan("free");

export const getPlan = (id: PlanId) => {
	const plan = PLANS.find((entry) => entry.id === id);
	if (!plan) {
		throw new Error(`unknown plan: ${id}`);
	}
	return plan;
};

/** Rivet project (pallet) plans. A plan sets the engine node count and SLA. */
export const PALLET_PLANS = [
	{
		id: "free",
		monthlyUsd: 0,
		description: "For prototyping.",
		nodes: 1,
		sla: null,
		highlights: ["1 node", "No SLA", "Community support"],
	},
	{
		id: "pro",
		monthlyUsd: 20,
		description: "For side projects that need to stay up.",
		nodes: 1,
		sla: "99.5%",
		highlights: ["1 node", "99.5% uptime SLA", "Email support"],
	},
	{
		id: "team",
		monthlyUsd: 200,
		description: "For production workloads and growing teams.",
		nodes: 3,
		sla: "99.9%",
		highlights: [
			"3 nodes, high availability",
			"99.9% uptime SLA",
			"Slack & email support",
		],
	},
	{
		id: "enterprise",
		monthlyUsd: 1500,
		description: "For compliance, SSO, audit logs, and dedicated support.",
		nodes: 3,
		sla: "99.99%",
		highlights: [
			"3 nodes, high availability",
			"99.99% uptime SLA",
			"Dedicated support",
		],
	},
] as const;

/** Engine node sizes, keyed by the cloud API v2 `node_size`. */
export const PALLET_NODE_SIZES = [
	{
		id: "small",
		label: "R-10",
		vcpu: 1 / 8,
		memoryGb: 1,
		storageGb: 20,
		monthlyUsdPerNode: 8,
	},
	{
		id: "medium",
		label: "R-40",
		vcpu: 1 / 2,
		memoryGb: 4,
		storageGb: 80,
		monthlyUsdPerNode: 26,
	},
	{
		id: "large",
		label: "R-160",
		vcpu: 2,
		memoryGb: 16,
		storageGb: 320,
		monthlyUsdPerNode: 90,
	},
] as const satisfies readonly {
	id: Rivet.v2.RegionsUpsertRequest.NodeSize;
	[key: string]: unknown;
}[];

/** Clouds a project can run on. `multiplier` scales node prices against AWS. */
export const PALLET_CLOUDS = [
	{
		id: "aws",
		label: "AWS",
		description: "Graviton instances.",
		multiplier: 1,
		regions: [
			{ id: "us-east-1", city: "N. Virginia", country: "US" },
			{ id: "us-west-2", city: "Oregon", country: "US" },
			{ id: "eu-west-1", city: "Ireland", country: "IE" },
			{ id: "eu-central-1", city: "Frankfurt", country: "DE" },
			{ id: "ap-southeast-1", city: "Singapore", country: "SG" },
			{ id: "ap-northeast-1", city: "Tokyo", country: "JP" },
		],
	},
	{
		id: "gcp",
		label: "Google Cloud",
		description: "Tau T2A machine family.",
		multiplier: 1.05,
		regions: [
			{ id: "us-central1", city: "Iowa", country: "US" },
			{ id: "us-east4", city: "N. Virginia", country: "US" },
			{ id: "europe-west1", city: "Belgium", country: "BE" },
			{ id: "europe-west3", city: "Frankfurt", country: "DE" },
			{ id: "asia-southeast1", city: "Singapore", country: "SG" },
			{ id: "asia-east1", city: "Taiwan", country: "TW" },
		],
	},
	{
		id: "azure",
		label: "Azure",
		description: "Ampere Altra series.",
		multiplier: 1.1,
		regions: [
			{ id: "eastus", city: "Virginia", country: "US" },
			{ id: "westus2", city: "Washington", country: "US" },
			{ id: "westeurope", city: "Netherlands", country: "NL" },
			{ id: "northeurope", city: "Ireland", country: "IE" },
			{ id: "southeastasia", city: "Singapore", country: "SG" },
			{ id: "japaneast", city: "Tokyo", country: "JP" },
		],
	},
] as const satisfies readonly {
	id: Rivet.v2.RegionsUpsertRequest.Cloud;
	[key: string]: unknown;
}[];

/** Allowances included with every project and their overage rates. */
export const PALLET_ALLOWANCES = {
	includedEgressGb: 100,
	egressUsdPerGb: 0.06,
	extraStorageUsdPerGb: 0.25,
	objectStorageMultiplier: 5,
	extraObjectStorageUsdPerGb: 0.02,
} as const;
