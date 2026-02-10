import { AxisBottom } from "@visx/axis";
import { Brush } from "@visx/brush";
import type BaseBrush from "@visx/brush/lib/BaseBrush";
import type { BrushHandleRenderProps } from "@visx/brush/lib/BrushHandle";
import { curveMonotoneX } from "@visx/curve";
import { Group } from "@visx/group";
import { PatternLines } from "@visx/pattern";
import { ParentSize } from "@visx/responsive";
import { scaleLinear, scaleTime } from "@visx/scale";
import { AreaClosed } from "@visx/shape";
import { extent } from "d3-array";
import { format } from "date-fns";
import { useCallback, useMemo, useRef } from "react";
import { useChartSync } from "./chart-sync-context";
import type { VisxAreaChartSeries } from "./visx-area-chart";

interface VisxBrushChartProps {
	series: VisxAreaChartSeries[];
	height?: number;
}

const BRUSH_HEIGHT = 50;
const MARGIN = { top: 5, right: 10, bottom: 20, left: 70 };

function BrushHandle({ x, height, isBrushActive }: BrushHandleRenderProps) {
	if (!isBrushActive) return null;
	const pathWidth = 6;
	const pathHeight = 16;
	return (
		<Group left={x - pathWidth / 2} top={(height - pathHeight) / 2}>
			<rect
				width={pathWidth}
				height={pathHeight}
				rx={3}
				fill="hsl(var(--muted-foreground))"
				fillOpacity={0.6}
			/>
		</Group>
	);
}

function BrushChart({
	series,
	width,
	height,
}: VisxBrushChartProps & { width: number }) {
	const brushRef = useRef<BaseBrush | null>(null);
	const { brushDomain, setBrushDomain } = useChartSync();

	const innerWidth = width - MARGIN.left - MARGIN.right;
	const innerHeight = height! - MARGIN.top - MARGIN.bottom;

	const allDates = useMemo(
		() => series.flatMap((s) => s.data.map((d) => d.ts)),
		[series],
	);
	const allValues = useMemo(
		() => series.flatMap((s) => s.data.map((d) => d.value)),
		[series],
	);

	const [minDate, maxDate] = useMemo(() => {
		const [min, max] = extent(allDates);
		return [min ?? new Date(), max ?? new Date()];
	}, [allDates]);

	const xScale = useMemo(
		() =>
			scaleTime({
				domain: [minDate, maxDate],
				range: [0, innerWidth],
			}),
		[minDate, maxDate, innerWidth],
	);

	const yScale = useMemo(() => {
		const maxVal = Math.max(...allValues, 0);
		return scaleLinear({
			domain: [0, maxVal * 1.1 || 1],
			range: [innerHeight, 0],
		});
	}, [allValues, innerHeight]);

	const initialBrushPosition = useMemo(() => {
		if (!brushDomain) return undefined;
		return {
			start: { x: xScale(brushDomain[0]) },
			end: { x: xScale(brushDomain[1]) },
		};
	}, [brushDomain, xScale]);

	const onBrushEnd = useCallback(
		(domain: { x0: number; x1: number; y0: number; y1: number } | null) => {
			if (!domain) {
				setBrushDomain(null);
				return;
			}
			const { x0, x1 } = domain;
			const start = new Date(x0);
			const end = new Date(x1);
			if (end.getTime() - start.getTime() < 1000) {
				setBrushDomain(null);
				return;
			}
			setBrushDomain([start, end]);
		},
		[setBrushDomain],
	);

	const handleReset = useCallback(() => {
		if (brushRef.current) {
			brushRef.current.reset();
		}
		setBrushDomain(null);
	}, [setBrushDomain]);

	if (innerWidth <= 0 || innerHeight <= 0) return null;

	return (
		<div className="relative">
			<svg width={width} height={height}>
				<PatternLines
					id="brush-pattern"
					height={8}
					width={8}
					stroke="hsl(var(--chart-1))"
					strokeWidth={1}
					orientation={["diagonal"]}
				/>
				<Group top={MARGIN.top} left={MARGIN.left}>
					{series.map((s) => (
						<AreaClosed
							key={s.key}
							data={s.data}
							x={(d) => xScale(d.ts)}
							y={(d) => yScale(d.value)}
							yScale={yScale}
							fill={s.color}
							fillOpacity={0.15}
							curve={curveMonotoneX}
						/>
					))}

					<AxisBottom
						top={innerHeight}
						scale={xScale}
						tickFormat={(d) => format(d as Date, "HH:mm")}
						stroke="transparent"
						tickStroke="transparent"
						tickLabelProps={{
							fill: "hsl(var(--muted-foreground))",
							fontSize: 10,
							textAnchor: "middle",
						}}
						numTicks={5}
					/>

					<Brush
						xScale={xScale}
						yScale={yScale}
						width={innerWidth}
						height={innerHeight}
						margin={{ top: 0, right: 0, bottom: 0, left: 0 }}
						handleSize={8}
						innerRef={brushRef}
						resizeTriggerAreas={["left", "right"]}
						brushDirection="horizontal"
						initialBrushPosition={initialBrushPosition}
						onBrushEnd={onBrushEnd}
						selectedBoxStyle={{
							fill: "url(#brush-pattern)",
							stroke: "hsl(var(--chart-1))",
							strokeWidth: 1,
						}}
						renderBrushHandle={(props) => (
							<BrushHandle {...props} />
						)}
					/>
				</Group>
			</svg>

			{brushDomain && (
				<button
					type="button"
					onClick={handleReset}
					className="absolute top-0 right-3 text-xs text-muted-foreground hover:text-foreground transition-colors"
				>
					Reset zoom
				</button>
			)}
		</div>
	);
}

export function VisxBrushChart(props: VisxBrushChartProps) {
	const height = props.height ?? BRUSH_HEIGHT + MARGIN.top + MARGIN.bottom;
	return (
		<ParentSize debounceTime={100}>
			{({ width }) => {
				if (width <= 0) return null;
				return <BrushChart {...props} width={width} height={height} />;
			}}
		</ParentSize>
	);
}
