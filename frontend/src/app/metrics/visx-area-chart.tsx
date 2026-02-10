import { AxisBottom, AxisLeft } from "@visx/axis";
import { curveMonotoneX } from "@visx/curve";
import { localPoint } from "@visx/event";
import { GridRows } from "@visx/grid";
import { Group } from "@visx/group";
import { ParentSize } from "@visx/responsive";
import { scaleLinear, scaleTime } from "@visx/scale";
import { AreaClosed, LinePath } from "@visx/shape";
import { TooltipWithBounds, useTooltip } from "@visx/tooltip";
import { bisector, extent } from "d3-array";
import { format } from "date-fns";
import { Fragment, useCallback, useId, useMemo } from "react";
import { useChartSync } from "./chart-sync-context";

export interface VisxAreaChartSeries {
	key: string;
	color: string;
	data: Array<{ ts: Date; value: number }>;
}

interface VisxAreaChartProps {
	series: VisxAreaChartSeries[];
	formatValue: (value: number) => string;
	height?: number;
}

interface TooltipData {
	date: Date;
	items: Array<{ key: string; color: string; value: number }>;
}

const MARGIN = { top: 10, right: 10, bottom: 30, left: 70 };

const bisectDate = bisector<{ ts: Date; value: number }, Date>(
	(d) => d.ts,
).left;

function Chart({
	series,
	formatValue,
	width,
	height,
}: VisxAreaChartProps & { width: number }) {
	const chartId = useId();
	const { hoveredTimestamp, setHoveredTimestamp, brushDomain } = useChartSync();

	const innerWidth = width - MARGIN.left - MARGIN.right;
	const innerHeight = height! - MARGIN.top - MARGIN.bottom;

	const {
		showTooltip,
		hideTooltip,
		tooltipData,
		tooltipLeft,
		tooltipTop,
		tooltipOpen,
	} = useTooltip<TooltipData>();

	// Compute the effective X domain, respecting brush selection.
	const xDomain = useMemo(() => {
		if (brushDomain) return brushDomain;
		const allDates = series.flatMap((s) => s.data.map((d) => d.ts));
		const [min, max] = extent(allDates);
		if (!min || !max) return [new Date(), new Date()] as [Date, Date];
		return [min, max] as [Date, Date];
	}, [series, brushDomain]);

	const xScale = useMemo(
		() =>
			scaleTime({
				domain: xDomain,
				range: [0, innerWidth],
			}),
		[xDomain, innerWidth],
	);

	// Filter data within the visible domain.
	const filteredSeries = useMemo(() => {
		const [start, end] = xDomain;
		return series.map((s) => ({
			...s,
			data: s.data.filter((d) => d.ts >= start && d.ts <= end),
		}));
	}, [series, xDomain]);

	const yScale = useMemo(() => {
		const allValues = filteredSeries.flatMap((s) => s.data.map((d) => d.value));
		const maxVal = Math.max(...allValues, 0);
		return scaleLinear({
			domain: [0, maxVal * 1.1 || 1],
			range: [innerHeight, 0],
			nice: true,
		});
	}, [filteredSeries, innerHeight]);

	const handleMouseMove = useCallback(
		(event: React.MouseEvent<SVGRectElement>) => {
			const point = localPoint(event);
			if (!point) return;

			const x = point.x - MARGIN.left;
			const date = xScale.invert(x);
			const timestamp = date.getTime();

			setHoveredTimestamp(timestamp);

			const items: TooltipData["items"] = [];
			for (const s of filteredSeries) {
				if (s.data.length === 0) continue;
				const idx = bisectDate(s.data, date, 1);
				const d0 = s.data[idx - 1];
				const d1 = s.data[idx];
				let d = d0;
				if (d1 && d0) {
					d =
						date.getTime() - d0.ts.getTime() >
						d1.ts.getTime() - date.getTime()
							? d1
							: d0;
				}
				if (d) {
					items.push({ key: s.key, color: s.color, value: d.value });
				}
			}

			showTooltip({
				tooltipData: { date, items },
				tooltipLeft: point.x,
				tooltipTop: point.y,
			});
		},
		[xScale, filteredSeries, setHoveredTimestamp, showTooltip],
	);

	const handleMouseLeave = useCallback(() => {
		setHoveredTimestamp(null);
		hideTooltip();
	}, [setHoveredTimestamp, hideTooltip]);

	// Compute crosshair X position from shared hover state.
	const crosshairX = useMemo(() => {
		if (hoveredTimestamp == null) return null;
		const x = xScale(new Date(hoveredTimestamp));
		if (x < 0 || x > innerWidth) return null;
		return x;
	}, [hoveredTimestamp, xScale, innerWidth]);

	if (innerWidth <= 0 || innerHeight <= 0) return null;

	return (
		<div className="relative">
			<svg width={width} height={height}>
				<Group top={MARGIN.top} left={MARGIN.left}>
					<GridRows
						scale={yScale}
						width={innerWidth}
						stroke="hsl(var(--border))"
						strokeOpacity={0.5}
						numTicks={5}
					/>

					{filteredSeries.map((s) => (
						<Fragment key={s.key}>
							<defs>
								<linearGradient
									id={`gradient-${chartId}-${s.key}`}
									x1="0"
									y1="0"
									x2="0"
									y2="1"
								>
									<stop
										offset="5%"
										stopColor={s.color}
										stopOpacity={0.4}
									/>
									<stop
										offset="95%"
										stopColor={s.color}
										stopOpacity={0.05}
									/>
								</linearGradient>
							</defs>
							<AreaClosed
								data={s.data}
								x={(d) => xScale(d.ts)}
								y={(d) => yScale(d.value)}
								yScale={yScale}
								fill={`url(#gradient-${chartId}-${s.key})`}
								curve={curveMonotoneX}
							/>
							<LinePath
								data={s.data}
								x={(d) => xScale(d.ts)}
								y={(d) => yScale(d.value)}
								stroke={s.color}
								strokeWidth={2}
								curve={curveMonotoneX}
							/>
						</Fragment>
					))}

					<AxisBottom
						top={innerHeight}
						scale={xScale}
						tickFormat={(d) => format(d as Date, "HH:mm")}
						stroke="transparent"
						tickStroke="transparent"
						tickLabelProps={{
							fill: "hsl(var(--muted-foreground))",
							fontSize: 12,
							textAnchor: "middle",
						}}
						numTicks={5}
					/>
					<AxisLeft
						scale={yScale}
						tickFormat={(v) => formatValue(v as number)}
						stroke="transparent"
						tickStroke="transparent"
						tickLabelProps={{
							fill: "hsl(var(--muted-foreground))",
							fontSize: 12,
							textAnchor: "end",
							dy: "0.33em",
						}}
						numTicks={5}
					/>

					{crosshairX != null && (
						<line
							x1={crosshairX}
							x2={crosshairX}
							y1={0}
							y2={innerHeight}
							stroke="hsl(var(--muted-foreground))"
							strokeDasharray="4 2"
							strokeWidth={1}
							pointerEvents="none"
						/>
					)}

					<rect
						width={innerWidth}
						height={innerHeight}
						fill="transparent"
						onMouseMove={handleMouseMove}
						onMouseLeave={handleMouseLeave}
					/>
				</Group>
			</svg>

			{tooltipOpen && tooltipData && (
				<TooltipWithBounds
					top={tooltipTop}
					left={tooltipLeft}
					style={{
						position: "absolute",
						background: "hsl(var(--background))",
						border: "1px solid hsl(var(--border) / 0.5)",
						borderRadius: "0.5rem",
						boxShadow: "0 20px 25px -5px rgb(0 0 0 / 0.1)",
						padding: "0.375rem 0.625rem",
						fontSize: "0.75rem",
						lineHeight: "1.25rem",
						color: "hsl(var(--foreground))",
						pointerEvents: "none",
						zIndex: 10,
					}}
				>
					<div style={{ fontWeight: 500, marginBottom: "0.25rem" }}>
						{format(tooltipData.date, "PPp")}
					</div>
					{tooltipData.items.map((item) => (
						<div
							key={item.key}
							style={{
								display: "flex",
								alignItems: "center",
								gap: "0.5rem",
							}}
						>
							<div
								style={{
									width: "0.625rem",
									height: "0.625rem",
									borderRadius: "2px",
									backgroundColor: item.color,
									flexShrink: 0,
								}}
							/>
							<span
								style={{
									color: "hsl(var(--muted-foreground))",
								}}
							>
								{item.key}
							</span>
							<span
								style={{
									fontFamily: "monospace",
									fontWeight: 500,
									fontVariantNumeric: "tabular-nums",
									marginLeft: "auto",
								}}
							>
								{formatValue(item.value)}
							</span>
						</div>
					))}
				</TooltipWithBounds>
			)}
		</div>
	);
}

export function VisxAreaChart(props: VisxAreaChartProps) {
	const height = props.height ?? 200;
	return (
		<ParentSize debounceTime={100}>
			{({ width }) => {
				if (width <= 0) return null;
				return <Chart {...props} width={width} height={height} />;
			}}
		</ParentSize>
	);
}
