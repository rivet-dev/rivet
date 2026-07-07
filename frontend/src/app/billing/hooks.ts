import type { Rivet } from "@rivet-gg/cloud";
import { useQuery } from "@tanstack/react-query";
import { endOfMonth, startOfMonth } from "date-fns";
import { sumComputeCost } from "@/app/metrics/compute-cost";
import { COMPUTE_METRICS } from "@/app/metrics/constants";
import { useCloudProjectDataProvider } from "@/components/actors";
import { features } from "@/lib/features";

// Metered usage math lives on the backend: `GET /projects/{id}/billing/usage`
// returns the fully-computed breakdown (plan, cycle, per-metric usage/included/
// overage, total, highest percent). The dashboard only renders it. Compute cost
// is the exception: it comes from the project compute-metrics endpoint and is
// shown separately via `ComputeUsageCard` (backend `totalCents` already includes
// it, so the card is a breakdown, not an addend to the displayed total).

export type BillingUsage = Rivet.BillingUsageResponse;
export type BilledMetricUsage = Rivet.BilledMetricUsage;

// Bucket size (seconds) for the month-to-date compute cost query. Cost is an
// active-time-weighted sum, so the total is correct at any resolution; this
// only bounds the number of returned buckets.
const COMPUTE_COST_RESOLUTION = 800;

/** Fetch the computed billing usage breakdown for the current project. */
export function useBillingUsage(): BillingUsage | undefined {
	const dataProvider = useCloudProjectDataProvider();
	const { data } = useQuery({
		...dataProvider.currentProjectBillingUsageQueryOptions(),
	});
	return data;
}

/** Per-metric usage keyed by metric name, for direct lookup by the usage cards. */
export function billedMetricsMap(
	usage: BillingUsage,
): Map<string, BilledMetricUsage> {
	return new Map(usage.metrics.map((m) => [m.metric, m]));
}

// Aggregate this project's month-to-date compute cost (in dollars) from the
// project compute metrics endpoint. Compute is billed per active second by
// configured CPU and memory, so this sums active_seconds *
// computeCostPerSecond(cpu, memory) across buckets. Project-scoped. See
// @/app/metrics/compute-cost.
export function useBilledComputeCost() {
	const dataProvider = useCloudProjectDataProvider();
	const now = new Date();
	const { data, isLoading, isError, error } = useQuery({
		...dataProvider.currentProjectComputeMetricsQueryOptions({
			name: COMPUTE_METRICS,
			startAt: startOfMonth(now).toISOString(),
			endAt: endOfMonth(now).toISOString(),
			resolution: COMPUTE_COST_RESOLUTION,
		}),
		// Compute is only billed where the Compute feature is enabled.
		enabled: features.compute,
	});

	// A project with no compute pools 404s, and one that has pools but no
	// recorded usage returns an empty columnar result. In both cases the
	// project isn't using compute, so the billing page omits the compute card
	// entirely. Other errors (e.g. a transient 500) keep the card so it can
	// surface an error state rather than silently hiding billing info.
	const isNotFound =
		isError &&
		(error as { statusCode?: number } | null)?.statusCode === 404;
	const isEmpty = !!data && data.name.length === 0;

	return {
		monthToDate: sumComputeCost(data),
		isLoading,
		isError,
		isUnavailable: isNotFound || isEmpty,
	};
}

/** Highest per-metric usage percentage ("how close to the limit/bill"). */
export function useHighestUsagePercent(): number {
	return useBillingUsage()?.highestPercent ?? 0;
}
