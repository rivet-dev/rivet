import { format } from "date-fns";
import { useId } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "../ui/chart";
import { timing } from "../lib/timing";

interface ActorCpuStatsProps {
	interval?: number;
	cpu: number[];
	metricsAt: number;
	syncId?: string;
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
}: ActorCpuStatsProps) {
	const data = cpu.map((value, i) => ({
		x: `${(cpu.length - i) * -interval}`,
		value: value / 100,
		config: {
			label: new Date(
				metricsAt - (cpu.length - i) * timing.seconds(interval),
			),
		},
	}));

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
