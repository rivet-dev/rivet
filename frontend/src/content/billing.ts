import { faCheck, faPlus } from "@rivet-gg/icons";

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

/** Compute cost in dollars per active second for the given actor config. */
export function computeCostPerSecond(vcpus: number, memoryMb: number): number {
	return (
		vcpus * COMPUTE.cpuPerVcpuSecond +
		(memoryMb / 1024) * COMPUTE.memoryPerGibSecond
	);
}

export const PLANS = [
	{
		id: "free",
		title: "Free",
		price: "$0",
		features: [
			{ icon: faCheck, label: "1 vCPU Max" },
			{ icon: faCheck, label: "$5 /mo Compute Limit" },
			{ icon: faCheck, label: "5GiB Storage Limit" },
			{ icon: faCheck, label: "Community Support" },
			{ icon: faCheck, label: "5 Million Writes /mo Limit" },
			{ icon: faCheck, label: "200 Million Reads /mo Limit" },
			{ icon: faCheck, label: "100GiB Egress Limit" },
			{ icon: faCheck, label: "100,000 Awake Actors Hours Limit" },
		],
	},
	{
		id: "pro",
		title: "Hobby",
		price: "$20",
		usageBased: true,
		features: [
			{ icon: faPlus, label: "Up to 8 vCPU" },
			{ icon: faPlus, label: "25 Billion Reads /mo included" },
			{ icon: faPlus, label: "5GiB Storage included" },
			{ icon: faCheck, label: "Email Support" },
			{ icon: faPlus, label: "50 Million Writes /mo included" },
			{ icon: faPlus, label: "1TiB Egress included" },
			{ icon: faPlus, label: "400,000 Awake Actors Hours included" },
		],
	},
	{
		id: "team",
		title: "Team",
		price: "$200",
		usageBased: true,
		features: [
			{ icon: faPlus, label: "Up to 8 vCPU" },
			{ icon: faPlus, label: "25 Billion Reads /mo included" },
			{ icon: faPlus, label: "5GiB Storage included" },
			{ icon: faCheck, label: "Slack Support" },
			{ icon: faPlus, label: "50 Million Writes /mo included" },
			{ icon: faPlus, label: "1TiB Egress included" },
			{ icon: faPlus, label: "400,000 Awake Actors Hours included" },
			{ icon: faCheck, label: "MFA" },
		],
	},
	{
		id: "enterprise",
		title: "Enterprise",
		price: "Custom",
		custom: true,
		features: [
			{ icon: faCheck, label: "Everything in Team" },
			{ icon: faCheck, label: "Priority Support" },
			{ icon: faCheck, label: "SLA" },
			{ icon: faCheck, label: "OIDC SSO provider" },
			{ icon: faCheck, label: "Audit logs" },
			{ icon: faCheck, label: "Custom Roles" },
			{ icon: faCheck, label: "Device Tracking" },
			{ icon: faCheck, label: "Volume Pricing" },
		],
	},
] as const;

export type PlanId = (typeof PLANS)[number]["id"];

export const getPlan = (id: PlanId) => {
	const plan = PLANS.find((entry) => entry.id === id);
	if (!plan) {
		throw new Error(`unknown plan: ${id}`);
	}
	return plan;
};
