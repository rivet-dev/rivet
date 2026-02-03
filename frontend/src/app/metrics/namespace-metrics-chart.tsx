import type { Rivet } from "@rivet-gg/cloud";
import { format } from "date-fns";
import { useMemo } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import type { MetricConfig } from "./types";

interface NamespaceMetricsChartProps {
	syncId: string;
	metric: MetricConfig;
	metricsData: Rivet.namespaces.MetricsGetResponse | undefined;
	isLoading?: boolean;
	isError?: boolean;
}

const chartConfig: ChartConfig = {
	value: {
		label: "Value",
		color: "hsl(var(--chart-1))",
	},
};

export function NamespaceMetricsChart({
	syncId,
	metric,
	metricsData,
	isLoading,
	isError,
}: NamespaceMetricsChartProps) {
	const chartData = useMemo(() => {
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

		return dataPoints.sort(
			(a, b) => new Date(a.ts).getTime() - new Date(b.ts).getTime(),
		);
	}, [metricsData, metric.name]);

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

	if (chartData.length === 0) {
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
				<ChartContainer
					config={chartConfig}
					className="h-[200px] w-full"
				>
					<AreaChart data={chartData} syncId={syncId}>
						<CartesianGrid vertical={false} />
						<XAxis
							dataKey="ts"
							tickFormatter={(value) =>
								format(new Date(value), "HH:mm")
							}
							axisLine={false}
							tickLine={false}
							tick={{ fontSize: 12 }}
						/>
						<YAxis
							axisLine={false}
							tickLine={false}
							tick={{ fontSize: 12 }}
							tickFormatter={(value) => metric.formatValue(value)}
							width={70}
						/>
						<ChartTooltip
							content={
								<ChartTooltipContent
									hideIndicator
									labelFormatter={(label) =>
										format(new Date(label), "PPp")
									}
									valueFormatter={(value) =>
										metric.formatValue(Number(value))
									}
								/>
							}
						/>
						<defs>
							<linearGradient
								id={`fill-${metric.name}`}
								x1="0"
								y1="0"
								x2="0"
								y2="1"
							>
								<stop
									offset="5%"
									stopColor="var(--color-value)"
									stopOpacity={0.8}
								/>
								<stop
									offset="95%"
									stopColor="var(--color-value)"
									stopOpacity={0.1}
								/>
							</linearGradient>
						</defs>
						<Area
							dataKey="value"
							type="monotone"
							stroke="var(--color-value)"
							fill={`url(#fill-${metric.name})`}
							fillOpacity={0.4}
							strokeWidth={2}
							isAnimationActive={false}
						/>
					</AreaChart>
				</ChartContainer>
			</CardContent>
		</Card>
	);
}
