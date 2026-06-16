import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Content } from "@/app/layout";
import { H1 } from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { useHasManagedPool } from "@/components/actors/actor-details-shared";
import { ChartSyncProvider } from "./chart-sync-context";
import { ComputeMetricsChart } from "./compute-metrics-chart";
import {
	ALL_METRICS,
	COMPUTE_METRICS,
	COMPUTE_METRICS_CONFIG,
	METRICS_CONFIG,
} from "./constants";
import { OVERVIEW_RANGE_MS, OVERVIEW_RESOLUTION } from "./hooks";
import { NamespaceMetricsChart } from "./namespace-metrics-chart";

export function NamespaceMetricsPage() {
	const dataProvider = useCloudNamespaceDataProvider();
	const hasManagedPool = useHasManagedPool();

	const { startAt, endAt } = useMemo(() => {
		const now = new Date();
		return {
			startAt: new Date(now.getTime() - OVERVIEW_RANGE_MS).toISOString(),
			endAt: now.toISOString(),
		};
	}, []);

	const {
		data: overviewData,
		isLoading: overviewLoading,
		isError: overviewError,
	} = useQuery({
		...dataProvider.currentNamespaceMetricsQueryOptions({
			name: ALL_METRICS,
			startAt,
			endAt,
			resolution: OVERVIEW_RESOLUTION,
		}),
		refetchInterval: 30000,
	});

	const {
		data: computeOverviewData,
		isLoading: computeOverviewLoading,
		isError: computeOverviewError,
	} = useQuery({
		...dataProvider.currentNamespaceComputeMetricsQueryOptions({
			name: COMPUTE_METRICS,
			startAt,
			endAt,
			resolution: OVERVIEW_RESOLUTION,
		}),
		refetchInterval: 30000,
		// Only fetch compute metrics when the namespace actually uses Compute.
		enabled: hasManagedPool,
	});

	return (
		<Content>
			<div className="mb-4 pt-2 max-w-7xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
					<H1>Metrics</H1>
				</div>
				<p className="max-w-7xl mb-6 px-6 @6xl:px-0 text-muted-foreground">
					View real-time metrics for this namespace.
				</p>
			</div>

			<hr className="mb-6" />

			<ChartSyncProvider>
				<div className="px-4 max-w-7xl mx-auto @6xl:px-0 pb-8 grid grid-cols-1 lg:grid-cols-2 gap-6">
					{METRICS_CONFIG.map((metric) => (
						<NamespaceMetricsChart
							key={metric.name}
							metric={metric}
							overviewData={overviewData}
							overviewStartAt={startAt}
							overviewEndAt={endAt}
							isOverviewLoading={overviewLoading}
							isOverviewError={overviewError}
						/>
					))}
				</div>

				{hasManagedPool ? (
					<>
						<div className="px-6 @6xl:px-0 max-w-7xl mx-auto">
							<h2 className="text-lg font-semibold mb-1">
								Compute
							</h2>
							<p className="mb-6 text-muted-foreground">
								Usage and estimated cost for this namespace's
								Compute deployment.
							</p>
						</div>
						<div className="px-4 max-w-7xl mx-auto @6xl:px-0 pb-8 grid grid-cols-1 lg:grid-cols-2 gap-6">
							{COMPUTE_METRICS_CONFIG.map((metric) => (
								<ComputeMetricsChart
									key={metric.name}
									metric={metric}
									overviewData={computeOverviewData}
									overviewStartAt={startAt}
									overviewEndAt={endAt}
									isOverviewLoading={computeOverviewLoading}
									isOverviewError={computeOverviewError}
								/>
							))}
						</div>
					</>
				) : null}
			</ChartSyncProvider>
		</Content>
	);
}
