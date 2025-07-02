import { format } from "date-fns";
import { filesize } from "filesize";
import { useId } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "../ui/chart";
import { timing } from "../lib/timing";

interface ActorMemoryStatsProps {
	metricsAt: number;
	memory: number[];
	allocatedMemory?: number;
	syncId?: string;
	interval?: number;
}

const chartConfig = {
	value: {
		color: "hsl(var(--chart-1))",
		label: "Memory Usage",
	},
} satisfies ChartConfig;

export function ActorMemoryStats({
	interval = 15,
	memory,
	allocatedMemory,
	metricsAt,
	syncId,
}: ActorMemoryStatsProps) {
	const data = memory.map((value, i) => ({
		x: `${(memory.length - i) * -interval}`,
		value,
		config: {
			label: new Date(
				metricsAt - (memory.length - i) * timing.seconds(interval),
			),
		},
	}));

	const max = allocatedMemory || Math.max(...memory);

	const id = useId();

	const fillId = `fill-${id}`;
	return (
		<ChartContainer config={chartConfig} className="-ml-6">
			<AreaChart accessibilityLayer data={data} syncId={syncId}>
				<CartesianGrid vertical={true} />
				<XAxis
					interval="preserveStartEnd"
					dataKey="x"
					hide
					axisLine={false}
					domain={[0, 60]}
					tickCount={60}
					includeHidden
				/>
				<YAxis
					dataKey="value"
					axisLine={false}
					domain={[0, max]}
					tickFormatter={(value) =>
						`${Math.ceil((value / max) * 100)}%`
					}
				/>
				<ChartTooltip
					content={
						<ChartTooltipContent
							hideIndicator
							labelKey="label"
							labelFormatter={(label) => {
								return format(label, "HH:mm:ss");
							}}
							valueFormatter={(value) => {
								if (typeof value !== "number") {
									return "n/a";
								}
								return `${filesize(value)} (${Math.round((value / max) * 100).toFixed(2)}%)`;
							}}
						/>
					}
				/>
				<defs>
					<linearGradient id={fillId} x1="0" y1="0" x2="0" y2="1">
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
					isAnimationActive={false}
					dataKey="value"
					type="linear"
					fill={`url(#${fillId})`}
					fillOpacity={0.4}
					stroke="var(--color-value)"
					stackId="a"
				/>
			</AreaChart>
		</ChartContainer>
	);
}
