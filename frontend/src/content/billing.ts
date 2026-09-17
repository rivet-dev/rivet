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
