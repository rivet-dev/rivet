import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { NAMESPACE_COLORS } from "./constants";
import type { MetricConfig, NamespaceMetricsData } from "./types";
import { VisxAreaChart, type VisxAreaChartSeries } from "./visx-area-chart";
import { VisxBrushChart } from "./visx-brush-chart";
import { zeroFillDataPoints } from "./zero-fill";

interface MetricsChartProps {
	metric: MetricConfig;
	namespaces: string[];
	metricsData: Map<string, NamespaceMetricsData>;
	isLoading?: boolean;
	isError?: boolean;
	startAt: string;
	endAt: string;
	resolution: number;
}

export function MetricsChart({
	metric,
	namespaces,
	metricsData,
	isLoading,
	isError,
	startAt,
	endAt,
	resolution,
}: MetricsChartProps) {
	const series = useMemo<VisxAreaChartSeries[]>(() => {
		return namespaces.map((ns, index) => {
			const nsData = metricsData.get(ns);
			const metricData = nsData?.[metric.name] ?? [];
			const filled = zeroFillDataPoints(metricData, {
				startAt,
				endAt,
				resolution,
			});
			return {
				key: ns,
				color: NAMESPACE_COLORS[index % NAMESPACE_COLORS.length],
				data: filled.map((d) => ({
					ts: new Date(d.ts),
					value: d.value,
				})),
			};
		});
	}, [namespaces, metricsData, metric.name, startAt, endAt, resolution]);

	if (isLoading) {
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

	if (isError) {
		return (
			<Card>
				<CardHeader className="pb-2">
					<CardTitle className="text-base">{metric.title}</CardTitle>
					<p className="text-sm text-muted-foreground">
						{metric.description}
					</p>
				</CardHeader>
				<CardContent>
					<div className="h-[200px] flex items-center justify-center text-muted-foreground">
						Failed to load metrics
					</div>
				</CardContent>
			</Card>
		);
	}

	const hasData = series.some((s) => s.data.some((d) => d.value > 0));

	if (!hasData) {
		return (
			<Card>
				<CardHeader className="pb-2">
					<CardTitle className="text-base">{metric.title}</CardTitle>
					<p className="text-sm text-muted-foreground">
						{metric.description}
					</p>
				</CardHeader>
				<CardContent>
					<div className="h-[200px] flex items-center justify-center text-muted-foreground">
						No data available
					</div>
				</CardContent>
			</Card>
		);
	}

	return (
		<Card>
			<CardHeader className="pb-2">
				<CardTitle className="text-base">{metric.title}</CardTitle>
				<p className="text-sm text-muted-foreground">
					{metric.description}
				</p>
			</CardHeader>
			<CardContent>
				<VisxAreaChart
					series={series}
					formatValue={metric.formatValue}
				/>
				<VisxBrushChart series={series} />
				<div className="flex flex-wrap items-center justify-center gap-4 pt-3">
					{namespaces.map((ns, index) => (
						<div key={ns} className="flex items-center gap-1.5">
							<div
								className="h-2 w-2 shrink-0 rounded-[2px]"
								style={{
									backgroundColor:
										NAMESPACE_COLORS[
											index % NAMESPACE_COLORS.length
										],
								}}
							/>
							<span className="text-xs">{ns}</span>
						</div>
					))}
				</div>
			</CardContent>
		</Card>
	);
}
