import { format } from "date-fns";
import { filesize } from "filesize";
import { useId } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { timing } from "../lib/timing";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "../ui/chart";

interface ActorMemoryStatsProps {
	metricsAt: number;
	memory: number[];
	allocatedMemory?: number;
	syncId?: string;
	interval?: number;
	isRunning?: boolean;
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
	isRunning = true,
}: ActorMemoryStatsProps) {
	// Filter out trailing zeros in the last 15 seconds only if actor is still running
	let filteredMemory = [...memory];
	if (isRunning) {
		const secondsToCheck = 15;
		const pointsToCheck = Math.ceil(secondsToCheck / interval);

		// Find the last non-zero value and cut off any zeros after it
		for (
			let i = filteredMemory.length - 1;
			i >= Math.max(0, filteredMemory.length - pointsToCheck);
			i--
		) {
			if (filteredMemory[i] === 0) {
				filteredMemory = filteredMemory.slice(0, i);
			} else {
				break;
			}
		}
	}

	const data = filteredMemory.map((value, i) => ({
		x: `${(filteredMemory.length - i) * -interval}`,
		value,
		config: {
			label: new Date(
				metricsAt -
					(filteredMemory.length - i) * timing.seconds(interval),
			),
		},
	}));

	const max = allocatedMemory || Math.max(...filteredMemory);

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
