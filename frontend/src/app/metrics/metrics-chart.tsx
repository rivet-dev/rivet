import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useChartSync } from "./chart-sync-context";
import { NAMESPACE_COLORS } from "./constants";
import { OVERVIEW_RESOLUTION, useNamespaceDetailMetrics } from "./hooks";
import type { MetricConfig, NamespaceMetricsData } from "./types";
import { VisxAreaChart, type VisxAreaChartSeries } from "./visx-area-chart";
import { VisxBrushChart } from "./visx-brush-chart";
import { zeroFillDataPoints } from "./zero-fill";

interface MetricsChartProps {
	metric: MetricConfig;
	namespaces: string[];
	overviewData: Map<string, NamespaceMetricsData>;
	overviewStartAt: string;
	overviewEndAt: string;
	isOverviewLoading?: boolean;
	isOverviewError?: boolean;
}

function buildSeries(
	namespaces: string[],
	metricsData: Map<string, NamespaceMetricsData>,
	metricName: MetricConfig["name"],
	startAt: string,
	endAt: string,
	resolution: number,
): VisxAreaChartSeries[] {
	return namespaces.map((ns, index) => {
		const nsData = metricsData.get(ns);
		const metricData = nsData?.[metricName] ?? [];
		const filled = zeroFillDataPoints(metricData, { startAt, endAt, resolution });
		return {
			key: ns,
			color: NAMESPACE_COLORS[index % NAMESPACE_COLORS.length],
			data: filled.map((d) => ({ ts: new Date(d.ts), value: d.value })),
		};
	});
}


export function MetricsChart({
	metric,
	namespaces,
	overviewData,
	overviewStartAt,
	overviewEndAt,
	isOverviewLoading,
	isOverviewError,
}: MetricsChartProps) {
	const { brushDomain } = useChartSync();

	const {
		data: detailData,
		isLoading: isDetailLoading,
		isError: isDetailError,
		startAt: detailStartAt,
		endAt: detailEndAt,
		resolution: detailResolution,
	} = useNamespaceDetailMetrics({ namespaces, brushDomain });

	const overviewSeries = useMemo(
		() => buildSeries(namespaces, overviewData, metric.name, overviewStartAt, overviewEndAt, OVERVIEW_RESOLUTION),
		[namespaces, overviewData, metric.name, overviewStartAt, overviewEndAt],
	);

	const detailSeries = useMemo(
		() => buildSeries(namespaces, detailData, metric.name, detailStartAt, detailEndAt, detailResolution),
		[namespaces, detailData, metric.name, detailStartAt, detailEndAt, detailResolution],
	);

	if (isOverviewLoading) {
		return (
			<Card>
				<CardHeader className="pb-2">
					<Skeleton className="h-5 w-32" />
					<Skeleton className="h-4 w-48 mt-1" />
				</CardHeader>
				<CardContent>
					<Skeleton className="h-[200px]" />
				</CardContent>
			</Card>
		);
	}

	if (isOverviewError) {
		return (
			<Card>
				<CardHeader className="pb-2">
					<CardTitle className="text-base">{metric.title}</CardTitle>
					<p className="text-sm text-muted-foreground">{metric.description}</p>
				</CardHeader>
				<CardContent>
					<div className="h-[200px] flex items-center justify-center text-muted-foreground">
						Failed to load metrics
					</div>
				</CardContent>
			</Card>
		);
	}

	return (
		<Card>
			<CardHeader className="pb-2">
				<CardTitle className="text-base">{metric.title}</CardTitle>
				<p className="text-sm text-muted-foreground">{metric.description}</p>
			</CardHeader>
			<CardContent className="pb-0">
				<VisxAreaChart
					series={isDetailLoading || isDetailError ? overviewSeries : detailSeries}
					formatValue={metric.formatValue}
				/>
				<VisxBrushChart series={overviewSeries} />
				<div className="flex flex-wrap items-center justify-center gap-4 pt-3 pb-4">
					{namespaces.map((ns, index) => (
						<div key={ns} className="flex items-center gap-1.5">
							<div
								className="h-2 w-2 shrink-0 rounded-[2px]"
								style={{ backgroundColor: NAMESPACE_COLORS[index % NAMESPACE_COLORS.length] }}
							/>
							<span className="text-xs">{ns}</span>
						</div>
					))}
				</div>
			</CardContent>
		</Card>
	);
}
