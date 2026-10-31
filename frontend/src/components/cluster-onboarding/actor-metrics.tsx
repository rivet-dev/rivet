import { format } from "date-fns";
import { useId, useMemo } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "@/components/ui/chart";

/** Mock, deterministic per-product actor metrics for the onboarded view. */
export interface ActorMetrics {
	total: number;
	awake: number;
	connections: number;
	actionsPerMin: number;
	errors24h: number;
	stateBytes: number;
	wakeP50Ms: number;
	/** 24 h series, one point per 15 min. */
	awakeSeries: number[];
	actionsSeries: number[];
}

const POINTS = 96; // 24 h at 15 min

function seededRandom(seed: string) {
	let h = 2166136261;
	for (const ch of seed) {
		h ^= ch.charCodeAt(0);
		h = Math.imul(h, 16777619);
	}
	return () => {
		h ^= h << 13;
		h ^= h >>> 17;
		h ^= h << 5;
		return ((h >>> 0) % 10_000) / 10_000;
	};
}

export function mockActorMetrics(seed: string): ActorMetrics {
	const rand = seededRandom(seed);
	const total = 400 + Math.floor(rand() * 2400);
	// Diurnal curve with noise; awake is a fraction of total.
	const awakeSeries = Array.from({ length: POINTS }, (_, i) => {
		const t = i / POINTS;
		const diurnal = 0.5 + 0.5 * Math.sin((t - 0.3) * Math.PI * 2);
		const frac = 0.08 + 0.22 * diurnal + (rand() - 0.5) * 0.05;
		return Math.max(1, Math.round(total * frac));
	});
	const actionsSeries = awakeSeries.map((a) =>
		Math.round(a * (2.4 + rand() * 1.2)),
	);
	const awake = awakeSeries[POINTS - 1];
	return {
		total,
		awake,
		connections: Math.round(awake * (1.3 + rand() * 0.9)),
		actionsPerMin: actionsSeries[POINTS - 1],
		errors24h: Math.floor(rand() * 14),
		stateBytes: Math.round(total * (180_000 + rand() * 900_000)),
		wakeP50Ms: Math.round(18 + rand() * 40),
		awakeSeries,
		actionsSeries,
	};
}

export function formatBytes(bytes: number): string {
	if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
	if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(0)} MB`;
	return `${(bytes / 1e3).toFixed(0)} KB`;
}

export function ActorMetricsDashboard({ metrics }: { metrics: ActorMetrics }) {
	const now = useMemo(() => Date.now(), []);
	const stats: { label: string; value: string; hint?: string }[] = [
		{ label: "Actors", value: metrics.total.toLocaleString() },
		{
			label: "Awake",
			value: metrics.awake.toLocaleString(),
			hint: `${Math.round((metrics.awake / metrics.total) * 100)}% of actors`,
		},
		{
			label: "Sleeping",
			value: (metrics.total - metrics.awake).toLocaleString(),
		},
		{ label: "Connections", value: metrics.connections.toLocaleString() },
		{
			label: "Actions / min",
			value: metrics.actionsPerMin.toLocaleString(),
		},
		{
			label: "Errors (24h)",
			value: metrics.errors24h.toLocaleString(),
		},
		{ label: "State storage", value: formatBytes(metrics.stateBytes) },
		{ label: "Wake latency (p50)", value: `${metrics.wakeP50Ms} ms` },
	];

	return (
		<div className="flex flex-col gap-6">
			<dl className="grid grid-cols-2 gap-3 md:grid-cols-4">
				{stats.map((s) => (
					<div
						key={s.label}
						className="rounded-lg border bg-background/40 px-4 py-3"
					>
						<dt className="text-xs text-muted-foreground">
							{s.label}
						</dt>
						<dd className="mt-1 text-2xl font-semibold tabular-nums">
							{s.value}
						</dd>
						{s.hint ? (
							<dd className="text-xs text-muted-foreground">
								{s.hint}
							</dd>
						) : null}
					</div>
				))}
			</dl>
			<div className="grid gap-3 lg:grid-cols-2">
				<SeriesCard
					title="Awake actors"
					series={metrics.awakeSeries}
					now={now}
					label="Awake"
				/>
				<SeriesCard
					title="Actions per minute"
					series={metrics.actionsSeries}
					now={now}
					label="Actions"
				/>
			</div>
		</div>
	);
}

const chartConfig = {
	value: { color: "hsl(var(--chart-1))", label: "Value" },
} satisfies ChartConfig;

function SeriesCard({
	title,
	series,
	now,
	label,
}: {
	title: string;
	series: number[];
	now: number;
	label: string;
}) {
	const id = useId();
	const fillId = `fill-${id}`;
	const data = series.map((value, i) => ({
		x: i,
		value,
		at: new Date(now - (series.length - 1 - i) * 15 * 60_000),
	}));
	return (
		<div className="rounded-lg border bg-background/40 p-4">
			<div className="flex items-baseline justify-between">
				<h3 className="text-sm font-medium">{title}</h3>
				<span className="text-xs text-muted-foreground">Last 24h</span>
			</div>
			<ChartContainer
				config={{ value: { ...chartConfig.value, label } }}
				className="mt-3 h-40 w-full"
			>
				<AreaChart
					accessibilityLayer
					data={data}
					margin={{ left: -16, right: 0, top: 4, bottom: 0 }}
				>
					<CartesianGrid vertical={false} />
					<XAxis
						dataKey="x"
						hide
						axisLine={false}
						interval="preserveStartEnd"
					/>
					<YAxis
						dataKey="value"
						axisLine={false}
						tickLine={false}
						width={48}
						tickFormatter={(v: number) =>
							v >= 1000 ? `${(v / 1000).toFixed(1)}k` : `${v}`
						}
					/>
					<ChartTooltip
						content={
							<ChartTooltipContent
								hideIndicator
								labelKey="at"
								labelFormatter={(_, payload) => {
									const at = payload?.[0]?.payload?.at;
									return at ? format(at, "HH:mm") : "";
								}}
								valueFormatter={(v) =>
									typeof v === "number"
										? v.toLocaleString()
										: "n/a"
								}
							/>
						}
					/>
					<defs>
						<linearGradient id={fillId} x1="0" y1="0" x2="0" y2="1">
							<stop
								offset="5%"
								stopColor="var(--color-value)"
								stopOpacity={0.5}
							/>
							<stop
								offset="95%"
								stopColor="var(--color-value)"
								stopOpacity={0.05}
							/>
						</linearGradient>
					</defs>
					<Area
						isAnimationActive={false}
						dataKey="value"
						type="monotone"
						fill={`url(#${fillId})`}
						stroke="var(--color-value)"
						strokeWidth={1.5}
					/>
				</AreaChart>
			</ChartContainer>
		</div>
	);
}
