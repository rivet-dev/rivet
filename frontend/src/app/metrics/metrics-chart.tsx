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
import { NAMESPACE_COLORS } from "./constants";
import type { MetricConfig, NamespaceMetricsData } from "./types";

interface MetricsChartProps {
	syncId: string;
	metric: MetricConfig;
	namespaces: string[];
	metricsData: Map<string, NamespaceMetricsData>;
	isLoading?: boolean;
	isError?: boolean;
}

export function MetricsChart({
	syncId,
	metric,
	namespaces,
	metricsData,
	isLoading,
	isError,
}: MetricsChartProps) {
	const chartData = useMemo(() => {
		const timeMap = new Map<string, Record<string, number | string>>();

		namespaces.forEach((ns) => {
			const nsData = metricsData.get(ns);
			if (!nsData) return;

			const metricData = nsData[metric.name];
			if (!metricData) return;

			metricData.forEach(({ ts, value }) => {
				const existing = timeMap.get(ts) || { ts };
				existing[ns] = value;
				timeMap.set(ts, existing);
			});
		});

		return Array.from(timeMap.values()).sort(
			(a, b) =>
				new Date(a.ts as string).getTime() -
				new Date(b.ts as string).getTime(),
		);
	}, [namespaces, metricsData, metric.name]);

	const chartConfig = useMemo(() => {
		const config: ChartConfig = {};
		namespaces.forEach((ns, index) => {
			config[ns] = {
				label: ns,
				color: NAMESPACE_COLORS[index % NAMESPACE_COLORS.length],
			};
		});
		return config;
	}, [namespaces]);

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
					className="w-full h-[200px]"
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
									labelFormatter={(label) =>
										format(new Date(label), "PPp")
									}
									valueFormatter={(value) =>
										metric.formatValue(Number(value))
									}
								/>
							}
						/>
						{namespaces.map((ns, index) => (
							<Area
								key={ns}
								dataKey={ns}
								type="monotone"
								stroke={
									NAMESPACE_COLORS[
										index % NAMESPACE_COLORS.length
									]
								}
								fill={
									NAMESPACE_COLORS[
										index % NAMESPACE_COLORS.length
									]
								}
								fillOpacity={0.2}
								strokeWidth={2}
								isAnimationActive={false}
							/>
						))}
					</AreaChart>
				</ChartContainer>
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
