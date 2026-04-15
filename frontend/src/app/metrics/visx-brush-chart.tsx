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
import { MAX_BRUSH_RANGE_MS, useChartSync } from "./chart-sync-context";
import type { VisxAreaChartSeries } from "./visx-area-chart";

interface VisxBrushChartProps {
	series: VisxAreaChartSeries[];
	height?: number;
}

const BRUSH_HEIGHT = 50;
const MARGIN = { top: 5, right: 10, bottom: 26, left: 70 };

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

	// Compute the initial brush position once on first render with valid scale data.
	// We use a ref so that subsequent xScale changes (from data loading) don't reset
	// the brush position back to the default.
	const initialBrushPositionRef = useRef<{ start: { x: number }; end: { x: number } } | undefined>(undefined);
	if (initialBrushPositionRef.current === undefined && minDate < maxDate) {
		initialBrushPositionRef.current = {
			start: { x: xScale(brushDomain[0]) },
			end: { x: xScale(brushDomain[1]) },
		};
	}

	const onBrushEnd = useCallback(
		(domain: { x0: number; x1: number; y0: number; y1: number } | null) => {
			if (!domain) {
				setBrushDomain(null);
				return;
			}
			const { x0, x1 } = domain;
			let start = new Date(x0);
			const end = new Date(x1);
			if (end.getTime() - start.getTime() < 1000) {
				setBrushDomain(null);
				return;
			}
			if (end.getTime() - start.getTime() > MAX_BRUSH_RANGE_MS) {
				start = new Date(end.getTime() - MAX_BRUSH_RANGE_MS);
			}
			setBrushDomain([start, end]);
		},
		[setBrushDomain],
	);

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
						tickFormat={(d) => {
							const date = d as Date;
							const rangeMs = maxDate.getTime() - minDate.getTime();
							if (rangeMs > 2 * 24 * 60 * 60 * 1000) return format(date, "MMM d");
							return format(date, "MMM d HH:mm");
						}}
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
						margin={MARGIN}
						handleSize={8}
						innerRef={brushRef}
						resizeTriggerAreas={["left", "right"]}
						brushDirection="horizontal"
						initialBrushPosition={initialBrushPositionRef.current}
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
