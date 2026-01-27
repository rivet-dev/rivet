import type { Rivet } from "@rivet-gg/cloud";
import { useInfiniteQuery, useQueries, useQuery } from "@tanstack/react-query";
import { endOfMonth } from "date-fns";
import { useCloudProjectDataProvider } from "@/components/actors";
import { BILLING } from "@/content/billing";

const BILLED_METRICS = [
	"actor_awake",
	"kv_storage_used",
	"kv_read",
	"kv_write",
	"gateway_egress",
] as const;

export function useAggregatedMetrics() {
	const dataProvider = useCloudProjectDataProvider();
	const { data: namespaces } = useInfiniteQuery({
		...dataProvider.currentProjectNamespacesQueryOptions(),
	});
	const metricQueries = useQueries({
		queries:
			namespaces?.map((ns) => ({
				...dataProvider.currentProjectLatestMetricsQueryOptions({
					name: [...BILLED_METRICS],
					namespace: ns.name,
					endAt: endOfMonth(new Date()).toISOString(),
				}),
			})) ?? [],
	});

	const aggregated = metricQueries.reduce(
		(acc, query) => {
			if (query.data) {
				query.data.forEach((metric) => {
					if (!acc[metric.name]) {
						acc[metric.name] = 0n;
					}
					acc[metric.name] += metric.value;
				});
			}
			return acc;
		},
		{} as Record<Rivet.namespaces.MetricsGetRequestNameItem, bigint>,
	);
	return aggregated;
}

export function useHighestUsagePercent(): number {
	const dataProvider = useCloudProjectDataProvider();

	const { data: billingData } = useQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});

	const aggregated = useAggregatedMetrics();
	const plan = billingData?.billing.activePlan || "free";

	let highestPercent = 0;
	for (const key of BILLED_METRICS) {
		const current = aggregated[key] || 0n;
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
