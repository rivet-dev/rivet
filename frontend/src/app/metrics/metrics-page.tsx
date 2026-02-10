import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Content } from "@/app/layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { H1 } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { ChartSyncProvider } from "./chart-sync-context";
import { METRICS_CONFIG, TIME_RANGE_OPTIONS } from "./constants";
import { useNamespaceMetrics } from "./hooks";
import { MetricsChart } from "./metrics-chart";
import { MetricsTimeRangeSelect } from "./metrics-time-range-select";
import { NamespaceFilterCombobox } from "./namespace-filter-combobox";

export function MetricsPage() {
	const dataProvider = useCloudProjectDataProvider();

	const { data: namespaces } = useSuspenseInfiniteQuery({
		...dataProvider.currentProjectNamespacesQueryOptions(),
	});

	const [selectedNamespaces, setSelectedNamespaces] = useState<string[]>(() =>
		namespaces.map((ns) => ns.name),
	);
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
	} = useNamespaceMetrics({
		namespaces: selectedNamespaces,
		startAt,
		endAt,
		resolution,
	});

	return (
		<Content>
			<div className="mb-4 pt-2 max-w-7xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
					<SidebarToggle className="absolute left-4" />
					<H1>Metrics</H1>
					<div className="flex gap-4">
						<NamespaceFilterCombobox
							namespaces={namespaces ?? []}
							value={selectedNamespaces}
							onValueChange={setSelectedNamespaces}
						/>
						<MetricsTimeRangeSelect
							value={timeRange}
							onValueChange={setTimeRange}
						/>
					</div>
				</div>
				<p className="max-w-7xl mb-6 px-6 @6xl:px-0 text-muted-foreground">
					View real-time metrics across all namespaces in your
					project.
				</p>
			</div>

			<hr className="mb-6" />

			<ChartSyncProvider key={timeRange}>
				<div className="px-4 max-w-7xl mx-auto @6xl:px-0 pb-8 grid grid-cols-1 lg:grid-cols-2 gap-6">
					{METRICS_CONFIG.map((metric) => (
						<MetricsChart
							key={metric.name}
							metric={metric}
							namespaces={selectedNamespaces}
							metricsData={metricsData}
							isLoading={isLoading}
							isError={isError}
							startAt={startAt}
							endAt={endAt}
							resolution={resolution}
						/>
					))}
				</div>
			</ChartSyncProvider>
		</Content>
	);
}
