import { useId } from "react";
import {
	Area,
	AreaChart,
	CartesianGrid,
	ResponsiveContainer,
	Tooltip,
	XAxis,
	YAxis,
} from "recharts";

export function UsageChart() {
	const id = useId();
	return (
		<ResponsiveContainer width="100%" height={300}>
			<AreaChart data={[]}>
				<defs>
					<linearGradient
						id={`${id}-gradient`}
						x1="0"
						y1="0"
						x2="0"
						y2="1"
					>
						<stop
							offset="5%"
							stopColor="hsl(var(--primary))"
							stopOpacity={0.8}
						/>
						<stop
							offset="95%"
							stopColor="hsl(var(--primary))"
							stopOpacity={0}
						/>
					</linearGradient>
				</defs>
				<XAxis dataKey="name" />
				<YAxis />
				<CartesianGrid strokeDasharray="3 3" />
				<Tooltip />
				<Area
					type="monotone"
					dataKey="uv"
					stroke="hsl(var(--primary))"
					fillOpacity={1}
					fill={`url(#${id}-gradient)`}
				/>
			</AreaChart>
		</ResponsiveContainer>
	);
}
