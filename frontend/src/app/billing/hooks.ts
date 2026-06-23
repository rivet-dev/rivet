import type { Rivet } from "@rivet-gg/cloud";
import { useQuery } from "@tanstack/react-query";
import { endOfMonth, startOfMonth } from "date-fns";
import { useCloudProjectDataProvider } from "@/components/actors";
import { BILLING } from "@/content/billing";
import { features } from "@/lib/features";
import { COMPUTE_METRICS } from "@/app/metrics/constants";
import { sumComputeCost } from "@/app/metrics/compute-cost";

// Bucket size (seconds) for the month-to-date compute cost query. Cost is an
// active-time-weighted sum, so the total is correct at any resolution; this
// only bounds the number of returned buckets.
const COMPUTE_COST_RESOLUTION = 800;

const BILLED_METRICS = [
	"actor_awake",
	"kv_storage_used",
	"kv_read",
	"kv_write",
	"gateway_egress",
] satisfies Rivet.MetricName[];

export function useBilledMetrics() {
	const dataProvider = useCloudProjectDataProvider();
	const { data } = useQuery({
		...dataProvider.currentProjectLatestMetricsQueryOptions({
			name: BILLED_METRICS,
			endAt: endOfMonth(new Date()).toISOString(),
		}),
	});

	const aggregated: Record<(typeof BILLED_METRICS)[number], bigint> = {
		actor_awake: 0n,
		kv_storage_used: 0n,
		kv_read: 0n,
		kv_write: 0n,
		gateway_egress: 0n,
	};
	if (data) {
		for (const metric of data) {
			aggregated[metric.name as (typeof BILLED_METRICS)[number]] =
				metric.value;
		}
	}

	return aggregated;
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

export function useHighestUsagePercent(): number {
	const dataProvider = useCloudProjectDataProvider();

	const { data: billingData } = useQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});

	const aggregated = useBilledMetrics();
	const plan = billingData?.billing.activePlan || "free";

	let highestPercent = 0;
	for (const key of BILLED_METRICS) {
		const current = aggregated?.[key] || 0n;
		const included = BILLING.included[plan][key];
		if (included && included > 0n) {
			const percent = Number((current * 100n) / included);
			if (percent > highestPercent) {
				highestPercent = percent;
			}
		}
	}

	return highestPercent;
}
