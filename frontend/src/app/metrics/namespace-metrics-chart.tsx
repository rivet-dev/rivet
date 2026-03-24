import type { Rivet } from "@rivet-gg/cloud";
import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useChartSync } from "./chart-sync-context";
import { OVERVIEW_RESOLUTION, useSingleNamespaceDetailMetrics } from "./hooks";
import type { MetricConfig } from "./types";
import { VisxAreaChart, type VisxAreaChartSeries } from "./visx-area-chart";
import { VisxBrushChart } from "./visx-brush-chart";
import { zeroFillDataPoints } from "./zero-fill";

interface NamespaceMetricsChartProps {
	metric: MetricConfig;
	overviewData: Rivet.namespaces.MetricsGetResponse | undefined;
	overviewStartAt: string;
	overviewEndAt: string;
	isOverviewLoading?: boolean;
	isOverviewError?: boolean;
}

function parseRawDataPoints(metricsData: Rivet.namespaces.MetricsGetResponse | undefined, metricName: MetricConfig["name"]) {
	const dataPoints: { ts: string; value: number }[] = [];
	if (metricsData) {
		for (let i = 0; i < metricsData.name.length; i++) {
			if (metricsData.name[i] === metricName) {
				dataPoints.push({
					ts: `${String(metricsData.ts[i]).replace(" ", "T")}Z`,
					value: Number(metricsData.value[i]),
				});
			}
		}
	}
	return dataPoints;
}

function parseApiSeriesZeroFilled(
	metricsData: Rivet.namespaces.MetricsGetResponse | undefined,
	metricName: MetricConfig["name"],
	startAt: string,
	endAt: string,
	resolution: number,
): VisxAreaChartSeries[] {
	const filled = zeroFillDataPoints(parseRawDataPoints(metricsData, metricName), { startAt, endAt, resolution });
	return [{ key: "value", color: "hsl(var(--chart-1))", data: filled.map((d) => ({ ts: new Date(d.ts), value: d.value })) }];
}


export function NamespaceMetricsChart({
	metric,
	overviewData,
	overviewStartAt,
	overviewEndAt,
	isOverviewLoading,
	isOverviewError,
}: NamespaceMetricsChartProps) {
	const { brushDomain } = useChartSync();

	const {
		query: detailQuery,
		startAt: detailStartAt,
		endAt: detailEndAt,
		resolution: detailResolution,
	} = useSingleNamespaceDetailMetrics({ brushDomain });

	const overviewSeries = useMemo(
		() => parseApiSeriesZeroFilled(overviewData, metric.name, overviewStartAt, overviewEndAt, OVERVIEW_RESOLUTION),
		[overviewData, metric.name, overviewStartAt, overviewEndAt],
	);

	const detailSeries = useMemo(
		() => parseApiSeriesZeroFilled(detailQuery.data, metric.name, detailStartAt, detailEndAt, detailResolution),
		[detailQuery.data, metric.name, detailStartAt, detailEndAt, detailResolution],
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
			<CardContent className="pb-4">
				<VisxAreaChart
					series={detailQuery.isLoading || detailQuery.isError ? overviewSeries : detailSeries}
					formatValue={metric.formatValue}
				/>
				<VisxBrushChart series={overviewSeries} />
			</CardContent>
		</Card>
	);
}
