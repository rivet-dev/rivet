import { format } from "date-fns";
import { useId } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { timing } from "../lib/timing";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "../ui/chart";

interface ActorCpuStatsProps {
	interval?: number;
	cpu: number[];
	metricsAt: number;
	syncId?: string;
	isRunning?: boolean;
}

const chartConfig = {
	value: {
		color: "hsl(var(--chart-1))",
		label: "CPU Usage",
	},
} satisfies ChartConfig;

export function ActorCpuStats({
	interval = 15,
	cpu,
	metricsAt,
	syncId,
	isRunning = true,
}: ActorCpuStatsProps) {
	// Filter out trailing zeros in the last 15 seconds only if actor is still running
	let filteredCpu = [...cpu];
	if (isRunning) {
		const secondsToCheck = 15;
		const pointsToCheck = Math.ceil(secondsToCheck / interval);

		// Find the last non-zero value and cut off any zeros after it
		for (
			let i = filteredCpu.length - 1;
			i >= Math.max(0, filteredCpu.length - pointsToCheck);
			i--
		) {
			if (filteredCpu[i] === 0) {
				filteredCpu = filteredCpu.slice(0, i);
			} else {
				break;
			}
		}
	}

	const data = filteredCpu.map((value, i) => {
		let cpuPercent = 0;

		// Calculate CPU percentage using delta time between ticks
		if (i > 0) {
			const currentCpuTime = value;
			const previousCpuTime = filteredCpu[i - 1];
			const deltaTime = interval; // seconds between measurements

			// CPU percentage = (cpu_time_delta / time_delta) * 100
			// This gives us the percentage of CPU time used in the interval
			if (currentCpuTime >= previousCpuTime) {
				cpuPercent = Math.min(
					((currentCpuTime - previousCpuTime) / deltaTime) * 100,
					100,
				);
			}
		}

		return {
			x: `${(filteredCpu.length - i) * -interval}`,
			value: cpuPercent / 100, // Convert to 0-1 range for chart
			config: {
				label: new Date(
					metricsAt -
						(filteredCpu.length - i) * timing.seconds(interval),
				),
			},
		};
	});

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
				/>
				<YAxis
					dataKey="value"
					axisLine={false}
					domain={[0, 1]}
					tickFormatter={(value) => `${value * 100}%`}
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
								return `${(value * 100).toFixed(2)}%`;
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
