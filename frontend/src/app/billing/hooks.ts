import type { Rivet } from "@rivet-gg/cloud";
import { useInfiniteQuery, useQueries, useQuery } from "@tanstack/react-query";
import { endOfMonth, startOfMonth } from "date-fns";
import { MANAGED_SERVICES_POOL } from "@/app/managed-services";
import { useCloudProjectDataProvider } from "@/components/actors";
import { BILLING } from "@/content/billing";
import { features } from "@/lib/features";
import { COMPUTE_METRICS } from "@/app/metrics/constants";
import { sumComputeCost } from "@/app/metrics/compute-cost";

// Bucket size (seconds) for the month-to-date compute cost query. Cost is an
// active-time-weighted sum, so the total is correct at any resolution; this
// only bounds the number of returned buckets.
const COMPUTE_COST_RESOLUTION = 800;

// All billing math now lives on the backend: `GET /projects/{id}/billing/usage`
// returns the fully-computed breakdown (plan, cycle, per-metric usage/included/
// overage, total, highest percent). The dashboard only renders it, so the
// pricing rates and overage formulas have a single source of truth on the server
// and the frontend no longer depends on the shared `@rivetkit/billing-data`
// package.

export type BillingUsage = Rivet.BillingUsageResponse;
export type BilledMetricUsage = Rivet.BilledMetricUsage;

/** Fetch the computed billing usage breakdown for the current project. */
export function useBillingUsage(): BillingUsage | undefined {
	const dataProvider = useCloudProjectDataProvider();
	const { data } = useQuery({
		...dataProvider.currentProjectBillingUsageQueryOptions(),
	});
	return data;
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

/**
 * Whether any namespace in the project runs the managed services pool. Services
 * run on Rivet Compute, so the project is billed for compute even before any
 * usage has been recorded.
 */
export function useHasActiveManagedServices(): boolean {
	const dataProvider = useCloudProjectDataProvider();
	const { data: namespaces } = useInfiniteQuery({
		...dataProvider.currentProjectNamespacesQueryOptions(),
		enabled: features.compute,
	});
	const pools = useQueries({
		queries: (namespaces ?? []).map((ns) =>
			dataProvider.currentProjectManagedPoolQueryOptions({
				namespace: ns.name,
				pool: MANAGED_SERVICES_POOL,
				safe: true,
			}),
		),
	});
	return pools.some((pool) => pool.data?.status === "ready");
}

export function useHighestUsagePercent(): number {
	return useBillingUsage()?.highestPercent ?? 0;
}
