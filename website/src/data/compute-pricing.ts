// Configurable Rivet Compute pricing model.
//
// Rivet Compute meters usage per pool per minute: active instance-seconds, the
// vCPU count, and the memory (GiB) of each running instance. The monthly cost
// is the sum over those minutes of:
//
//   activeSeconds * (memoryRate * memoryGib + cpuRate * vcpu)
//
// Fill in the real `memoryRate` / `cpuRate` coefficients and any included
// allotment below, then flip `finalized` to true. While `finalized` is false
// (or any rate is left null), the docs render a beta notice instead of a rate
// table so no unverified numbers ship.

export interface ComputePriceDimension {
	/** Human-readable dimension name, e.g. "Memory". */
	label: string;
	/** What this dimension bills. */
	description: string;
	/** Unit price. Null until the rate is finalized. */
	rate: number | null;
	/** Unit the rate is expressed in, e.g. "per MiB-second". */
	unit: string;
}

export interface ComputeIncludedAllotment {
	/** Dimension the allotment applies to. */
	label: string;
	/** Included amount, e.g. "100 vCPU-hours / month". */
	amount: string;
}

export interface ComputePricingModel {
	/**
	 * Flip to true once every `rate` below is filled in with a real value. While
	 * false, the docs page renders a beta notice rather than asserting rates.
	 */
	finalized: boolean;
	/** Currency symbol prefixed to rendered rates. */
	currency: string;
	/** Billed dimensions that make up the compute cost. */
	dimensions: ComputePriceDimension[];
	/** Usage included before metered charges apply. Empty if there is none. */
	includedAllotments: ComputeIncludedAllotment[];
	/** One-line description of how and how often usage is billed. */
	billingGranularity: string;
}

export const computePricing: ComputePricingModel = {
	finalized: true,
	currency: "$",
	dimensions: [
		{
			label: "Memory",
			description: "Memory allocated to running instances.",
			rate: 0.0000029,
			unit: "per GiB-second",
		},
		{
			label: "vCPU",
			description: "Compute allocated to running instances.",
			rate: 0.000033,
			unit: "per vCPU-second",
		},
	],
	includedAllotments: [],
	billingGranularity:
		"Usage is metered per minute and billed monthly. You are only charged while instances are running.",
};
