"use client";
import { faArrowDownToLine, Icon } from "@rivet-gg/icons";
import type { Virtualizer } from "@tanstack/react-virtual";
import {
	type PropsWithChildren,
	type ReactNode,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { cn } from "./lib/utils";
import { Skeleton } from "./ui/skeleton";
import { Toggle } from "./ui/toggle";
import { WithTooltip } from "./ui/tooltip";
import { VirtualScrollArea } from "./virtual-scroll-area";

export function Root({ children }: PropsWithChildren) {
	return <div className="h-full">{children}</div>;
}

export function Content({ children }: PropsWithChildren) {
	return <div className="flex h-full gap-4">{children}</div>;
}

export function LogsArea({
	children,
	className,
}: PropsWithChildren<{ className?: string }>) {
	return (
		<div
			className={cn(
				"h-full rounded-lg border w-full flex gap-1",
				className,
			)}
		>
			{children}
		</div>
	);
}

export function Sidebar({ children }: PropsWithChildren) {
	return (
		<div className="flex flex-col gap-2 justify-between">{children}</div>
	);
}

type Line = string | { type: "log" | "error" | "warn"; message: string };

interface LogRowProps {
	timestamp?: string;
	line?: Line;
	isFirst?: boolean;
}

function LogRow({ timestamp, line, isFirst }: LogRowProps) {
	const isError = typeof line === "object" && line.type === "error";
	const isWarn = typeof line === "object" && line.type === "warn";

	return (
		<div className="text-nowrap flex flex-col md:flex-row my-1 md:my-0">
			{isFirst ? (
				<span className="font-mono text-xs">
					Only last few lines are visible here. To see all logs,
					export them.
				</span>
			) : (
				<>
					<span
						className={cn(
							"text-muted-foreground md:my-1 font-mono text-xs p-0.5 pr-2 inline-block",
							isError && "bg-destructive/20",
							isWarn && "bg-warning/20",
						)}
					>
						{timestamp}
					</span>
					<pre
						className={cn(
							"md:my-1 font-mono text-xs p-0.5 inline-block whitespace-pre-wrap min-w-0 flex-1 break-all",
							isError && "bg-destructive/20",
							isWarn && "bg-warning/20",
						)}
					>
						{typeof line === "string" ? line : line?.message}
					</pre>
				</>
			)}
		</div>
	);
}

interface LogsViewProps {
	timestamps: string[];
	lines: Line[];
	sidebar?: ReactNode;
	showFollowToggle?: boolean;
	showTurncatedLogsInfo?: boolean;
	empty?: ReactNode;
}

export function LogsView({
	sidebar,
	timestamps,
	lines,
	empty,
	showFollowToggle = true,
	showTurncatedLogsInfo = false,
}: LogsViewProps) {
	const [follow, setFollow] = useState(true);
	const isEmpty = lines.length === 0 || timestamps.length === 0;

	const viewport = useRef<HTMLDivElement>(null);
	const ref = useRef<Virtualizer<HTMLDivElement, Element>>(null);

	useEffect(() => {
		if (!follow) {
			return;
		}
		// https://github.com/TanStack/virtual/issues/537
		const rafId = requestAnimationFrame(() => {
			ref.current?.scrollToIndex(timestamps.length, { align: "end" });
		});

		return () => {
			cancelAnimationFrame(rafId);
		};
	}, [timestamps, follow]);

	const handleChange = useCallback(
		(instance: Virtualizer<HTMLDivElement, Element>) => {
			const isAtBottom =
				(instance.range?.endIndex || 0) >= timestamps.length - 1;

			if (isAtBottom) {
				return setFollow(true);
			}
			if (instance.scrollDirection === "backward") {
				return setFollow(false);
			}
		},
		[timestamps],
	);

	return (
		<Root>
			<Content>
				<LogsArea>
					{isEmpty ? (
						<div className="text-muted-foreground py-8 text-center self-center w-full">
							{empty ? (
								empty
							) : (
								<>
									<p>No logs available.</p>
									<p>
										Logs older than 48 hours will not show
										up here.
									</p>
								</>
							)}
						</div>
					) : (
						<VirtualScrollArea
							virtualizerRef={ref}
							viewportRef={viewport}
							onChange={handleChange}
							getRowData={(index) => ({
								timestamp: showTurncatedLogsInfo
									? timestamps[index - 1]
									: timestamps[index],
								line: showTurncatedLogsInfo
									? lines[index - 1]
									: lines[index],
								isFirst: index === 0 && showTurncatedLogsInfo,
							})}
							count={
								timestamps.length > 0 && showTurncatedLogsInfo
									? timestamps.length + 1
									: timestamps.length
							}
							paddingStart={8}
							paddingEnd={8}
							estimateSize={() => 28}
							className="w-full"
							row={LogRow}
						/>
					)}
				</LogsArea>
				{!sidebar && !showFollowToggle ? null : (
					<Sidebar>
						<div>{sidebar}</div>

						{showFollowToggle ? (
							<div className="border-t pt-4">
								<WithTooltip
									content="Follow logs"
									trigger={
										<div>
											<Toggle
												onPressedChange={setFollow}
												pressed={
													isEmpty ? false : follow
												}
												disabled={isEmpty}
												variant="outline"
												aria-label="Toggle follow logs"
											>
												<Icon
													className="size-4"
													icon={faArrowDownToLine}
												/>
											</Toggle>
										</div>
									}
								/>
							</div>
						) : null}
					</Sidebar>
				)}
			</Content>
		</Root>
	);
}

LogsView.Skeleton = function LogsViewSkeleton() {
	return (
		<Root>
			<Content>
				<LogsArea className="flex-col p-4">
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
					<Skeleton className="w-full h-6" />
				</LogsArea>
				<Sidebar>
					<Skeleton className="w-11 h-11" />
					<Skeleton className="w-11 h-11" />
				</Sidebar>
			</Content>
		</Root>
	);
};
