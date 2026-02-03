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
] satisfies Rivet.namespaces.MetricsGetRequestNameItem[];

export function useBilledMetrics() {
	const dataProvider = useCloudProjectDataProvider();
	const {data} = useQuery({
		...dataProvider.currentProjectLatestMetricsQueryOptions({
			name: BILLED_METRICS,
			endAt: endOfMonth(new Date()).toISOString(),
		})
	})

	const aggregated: Record<typeof BILLED_METRICS[number], bigint> = {
		actor_awake: 0n,
		kv_storage_used: 0n,
		kv_read: 0n,
		kv_write: 0n,
		gateway_egress: 0n
	};
	if (data) {
		for (const metric of data) {
			aggregated[metric.name as typeof BILLED_METRICS[number]] = metric.value;
		}
	}

	return aggregated;
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
