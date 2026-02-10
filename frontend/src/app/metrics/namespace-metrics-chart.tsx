import type { Rivet } from "@rivet-gg/cloud";
import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import type { MetricConfig } from "./types";
import { VisxAreaChart, type VisxAreaChartSeries } from "./visx-area-chart";
import { VisxBrushChart } from "./visx-brush-chart";
import { zeroFillDataPoints } from "./zero-fill";

interface NamespaceMetricsChartProps {
	metric: MetricConfig;
	metricsData: Rivet.namespaces.MetricsGetResponse | undefined;
	isLoading?: boolean;
	isError?: boolean;
	startAt: string;
	endAt: string;
	resolution: number;
}

export function NamespaceMetricsChart({
	metric,
	metricsData,
	isLoading,
	isError,
	startAt,
	endAt,
	resolution,
}: NamespaceMetricsChartProps) {
	const series = useMemo<VisxAreaChartSeries[]>(() => {
		if (!metricsData) return [];

		const dataPoints: { ts: string; value: number }[] = [];
		for (let i = 0; i < metricsData.name.length; i++) {
			if (metricsData.name[i] === metric.name) {
				dataPoints.push({
					ts: metricsData.ts[i],
					value: Number(metricsData.value[i]),
				});
			}
		}

		const filled = zeroFillDataPoints(dataPoints, {
			startAt,
			endAt,
			resolution,
		});

		return [
			{
				key: "value",
				color: "hsl(var(--chart-1))",
				data: filled.map((d) => ({
					ts: new Date(d.ts),
					value: d.value,
				})),
			},
		];
	}, [metricsData, metric.name, startAt, endAt, resolution]);

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
			</CardContent>
		</Card>
	);
}
