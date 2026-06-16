import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { computeCostPerSecond } from "@/content/billing";
import { useChartSync } from "./chart-sync-context";
import {
	OVERVIEW_RESOLUTION,
	useSingleNamespaceComputeDetailMetrics,
} from "./hooks";
import type {
	ComputeMetricConfig,
	ComputeMetricKey,
	MetricsColumnar,
} from "./types";
import { VisxAreaChart, type VisxAreaChartSeries } from "./visx-area-chart";
import { VisxBrushChart } from "./visx-brush-chart";
import { zeroFillDataPoints } from "./zero-fill";

interface ComputeMetricsChartProps {
	metric: ComputeMetricConfig;
	overviewData: MetricsColumnar | undefined;
	overviewStartAt: string;
	overviewEndAt: string;
	isOverviewLoading?: boolean;
	isOverviewError?: boolean;
}

// Normalize "YYYY-MM-DD HH:MM:SS" to ISO 8601 UTC so Date.parse works
// consistently across browsers and aligns with the generated buckets.
function normalizeTs(ts: string): string {
	return `${String(ts).replace(" ", "T")}Z`;
}

// Derive the per-bucket cost series from the raw active_seconds / cpu /
// memory_mib columns. Because the endpoint emits active-time-weighted cpu and
// memory, active_seconds * computeCostPerSecond(cpu, memory) per bucket equals
// the exact sum over individual instances. See @/content/billing.
function parseCostDataPoints(data: MetricsColumnar) {
	const byTs = new Map<
		string,
		{ activeSeconds: number; cpu: number; memoryMib: number }
	>();
	for (let i = 0; i < data.name.length; i++) {
		const ts = normalizeTs(data.ts[i]);
		const value = Number(data.value[i]);
		const entry = byTs.get(ts) ?? {
			activeSeconds: 0,
			cpu: 0,
			memoryMib: 0,
		};
		if (data.name[i] === "active_seconds") entry.activeSeconds = value;
		else if (data.name[i] === "cpu") entry.cpu = value;
		else if (data.name[i] === "memory_mib") entry.memoryMib = value;
		byTs.set(ts, entry);
	}

	return Array.from(byTs.entries())
		.map(([ts, { activeSeconds, cpu, memoryMib }]) => ({
			ts,
			value: activeSeconds * computeCostPerSecond(cpu, memoryMib),
		}))
		.sort((a, b) => a.ts.localeCompare(b.ts));
}

function parseRawDataPoints(
	data: MetricsColumnar | undefined,
	metricName: ComputeMetricKey,
) {
	if (!data) return [];
	if (metricName === "cost") return parseCostDataPoints(data);

	const dataPoints: { ts: string; value: number }[] = [];
	for (let i = 0; i < data.name.length; i++) {
		if (data.name[i] === metricName) {
			dataPoints.push({
				ts: normalizeTs(data.ts[i]),
				value: Number(data.value[i]),
			});
		}
	}
	return dataPoints;
}

function parseApiSeriesZeroFilled(
	data: MetricsColumnar | undefined,
	metricName: ComputeMetricKey,
	startAt: string,
	endAt: string,
	resolution: number,
): VisxAreaChartSeries[] {
	const filled = zeroFillDataPoints(parseRawDataPoints(data, metricName), {
		startAt,
		endAt,
		resolution,
	});
	return [
		{
			key: "value",
			color: "hsl(var(--chart-1))",
			data: filled.map((d) => ({ ts: new Date(d.ts), value: d.value })),
		},
	];
}

export function ComputeMetricsChart({
	metric,
	overviewData,
	overviewStartAt,
	overviewEndAt,
	isOverviewLoading,
	isOverviewError,
}: ComputeMetricsChartProps) {
	const { brushDomain } = useChartSync();

	const {
		query: detailQuery,
		startAt: detailStartAt,
		endAt: detailEndAt,
		resolution: detailResolution,
	} = useSingleNamespaceComputeDetailMetrics({ brushDomain });

	const overviewSeries = useMemo(
		() =>
			parseApiSeriesZeroFilled(
				overviewData,
				metric.name,
				overviewStartAt,
				overviewEndAt,
				OVERVIEW_RESOLUTION,
			),
		[overviewData, metric.name, overviewStartAt, overviewEndAt],
	);

	const detailSeries = useMemo(
		() =>
			parseApiSeriesZeroFilled(
				detailQuery.data,
				metric.name,
				detailStartAt,
				detailEndAt,
				detailResolution,
			),
		[
			detailQuery.data,
			metric.name,
			detailStartAt,
			detailEndAt,
			detailResolution,
		],
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

	return (
		<Card>
			<CardHeader className="pb-2">
				<CardTitle className="text-base">{metric.title}</CardTitle>
				<p className="text-sm text-muted-foreground">
					{metric.description}
				</p>
			</CardHeader>
			<CardContent className="pb-4">
				<VisxAreaChart
					series={
						detailQuery.isLoading || detailQuery.isError
							? overviewSeries
							: detailSeries
					}
					formatValue={metric.formatValue}
				/>
				<VisxBrushChart series={overviewSeries} />
			</CardContent>
		</Card>
	);
}
