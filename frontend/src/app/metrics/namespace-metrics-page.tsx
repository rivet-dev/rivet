import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Content } from "@/app/layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { H1 } from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { ChartSyncProvider } from "./chart-sync-context";
import { ALL_METRICS, METRICS_CONFIG } from "./constants";
import { OVERVIEW_RANGE_MS, OVERVIEW_RESOLUTION } from "./hooks";
import { NamespaceMetricsChart } from "./namespace-metrics-chart";

export function NamespaceMetricsPage() {
	const dataProvider = useCloudNamespaceDataProvider();

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

	return (
		<Content>
			<div className="mb-4 pt-2 max-w-7xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
					<SidebarToggle className="absolute left-4" />
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
			</ChartSyncProvider>
		</Content>
	);
}
