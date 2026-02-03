import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Content } from "@/app/layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { H1 } from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { ALL_METRICS, METRICS_CONFIG, TIME_RANGE_OPTIONS } from "./constants";
import { MetricsTimeRangeSelect } from "./metrics-time-range-select";
import { NamespaceMetricsChart } from "./namespace-metrics-chart";

const SYNC_ID = "namespace-metrics";

export function NamespaceMetricsPage() {
	const dataProvider = useCloudNamespaceDataProvider();

	const [timeRange, setTimeRange] = useState("1h");

	const { startAt, endAt, resolution } = useMemo(() => {
		const option = TIME_RANGE_OPTIONS.find((o) => o.value === timeRange);
		const now = new Date();
		return {
			startAt: new Date(
				now.getTime() - (option?.milliseconds ?? 3600000),
			).toISOString(),
			endAt: now.toISOString(),
			resolution: option?.resolution ?? 60,
		};
	}, [timeRange]);

	const {
		data: metricsData,
		isLoading,
		isError,
	} = useQuery({
		...dataProvider.currentNamespaceMetricsQueryOptions({
			name: ALL_METRICS,
			startAt,
			endAt,
			resolution,
		}),
		refetchInterval: 30000,
	});

	return (
		<Content>
			<div className="mb-4 pt-2 max-w-7xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
					<SidebarToggle className="absolute left-4" />
					<H1>Metrics</H1>
					<MetricsTimeRangeSelect
						value={timeRange}
						onValueChange={setTimeRange}
					/>
				</div>
				<p className="max-w-7xl mb-6 px-6 @6xl:px-0 text-muted-foreground">
					View real-time metrics for this namespace.
				</p>
			</div>

			<hr className="mb-6" />

			<div className="px-4 max-w-7xl mx-auto @6xl:px-0 pb-8 grid grid-cols-1 lg:grid-cols-2 gap-6">
				{METRICS_CONFIG.map((metric) => (
					<NamespaceMetricsChart
						key={metric.name}
						syncId={SYNC_ID}
						metric={metric}
						metricsData={metricsData}
						isLoading={isLoading}
						isError={isError}
					/>
				))}
			</div>
		</Content>
	);
}
