import type { RivetSse } from "@rivet-gg/cloud";
import { faTriangleExclamation, Icon } from "@rivet-gg/icons";
import type { Virtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useRef, useState } from "react";
import { ErrorDetails } from "@/components/actors";
import { VirtualScrollArea } from "@/components/virtual-scroll-area";
import { AnsiText } from "./lib/ansi";
import { cn } from "./lib/utils";
import { ScrollArea } from "./ui/scroll-area";
import { Skeleton } from "./ui/skeleton";
import { useDeploymentLogsStream } from "./use-deployment-logs-stream";

const SKELETON_KEYS = [
	"a",
	"b",
	"c",
	"d",
	"e",
	"f",
	"g",
	"h",
	"i",
	"j",
	"k",
	"l",
	"m",
	"n",
	"o",
	"p",
	"q",
	"r",
	"s",
	"t",
	"u",
	"v",
	"w",
	"x",
	"y",
	"z",
	"aa",
	"ab",
	"ac",
	"ad",
	"ae",
	"af",
	"ag",
	"ah",
	"ai",
	"aj",
	"ak",
	"al",
	"am",
	"an",
];

interface DeploymentLogsProps {
	project?: string;
	namespace?: string;
	pool: string;
	filter?: string;
	region?: string;
	paused?: boolean;
	logsRef?: React.MutableRefObject<RivetSse.LogStreamEvent.Log[]>;
}

interface LogRowData {
	className?: string;
	entry?: RivetSse.LogStreamEvent.Log;
	isSentinel?: boolean;
	isLoadingMore?: boolean;
}

function LogRow({ entry, isSentinel, isLoadingMore, ...props }: LogRowData) {
	if (isSentinel) {
		return (
			<div
				{...props}
				className={cn(
					"px-4 py-1 border-b text-muted-foreground/50 italic",
					props.className,
				)}
			>
				{isLoadingMore
					? "Loading older logs…"
					: "Scroll to top to load older logs"}
			</div>
		);
	}

	if (!entry) return null;

	return (
		<div
			{...props}
			className={cn("font-mono grid grid-cols-subgrid", props.className)}
		>
			<div
				className={cn(
					"grid grid-cols-[max-content,16ch,3fr] gap-3 whitespace-pre-wrap break-words px-4 py-1 border-b",
					{
						"text-red-400": entry.data.severity === "error",
						"text-muted-foreground": entry.data.severity !== "error",
					},
				)}
			>
				<span className="text-neutral-500 shrink-0">
					{entry.data.timestamp}
				</span>
				{entry.data.region ? (
					<span className="text-neutral-600 shrink-0">
						[{entry.data.region}]
					</span>
				) : null}
				<span className="flex-1">
					<AnsiText text={entry.data.message} />
				</span>
			</div>
		</div>
	);
}

export function DeploymentLogs({
	project,
	namespace,
	pool,
	filter,
	region,
	paused,
	logsRef,
}: DeploymentLogsProps) {
	const {
		logs,
		isLoading,
		error,
		streamError,
		isLoadingMore,
		hasMore,
		loadMoreHistory,
	} = useDeploymentLogsStream({ project: project ?? "", namespace: namespace ?? "", pool, filter, region, paused });

	const viewportRef = useRef<HTMLDivElement>(null);
	const virtualizerRef = useRef<Virtualizer<HTMLDivElement, Element>>(null);
	const [follow, setFollow] = useState(true);
	// Track the log count before a load-more so we can restore scroll position.
	const prevLogCountRef = useRef(0);

	// When hasMore, index 0 is the sentinel row; real logs start at index 1.
	const sentinelOffset = hasMore ? 1 : 0;
	const totalCount = logs.length + sentinelOffset;

	useEffect(() => {
		if (follow && !isLoading && virtualizerRef.current && logs.length > 0) {
			// https://github.com/TanStack/virtual/issues/537
			const rafId = requestAnimationFrame(() => {
				virtualizerRef.current?.scrollToIndex(totalCount - 1, {
					align: "end",
				});
			});
			return () => cancelAnimationFrame(rafId);
		}
	}, [totalCount, logs.length, follow, isLoading]);

	// After prepending older history, scroll to restore the previously-first row.
	const wasLoadingMoreRef = useRef(false);
	useEffect(() => {
		if (
			wasLoadingMoreRef.current &&
			!isLoadingMore &&
			logs.length > prevLogCountRef.current
		) {
			const addedCount = logs.length - prevLogCountRef.current;
			const rafId = requestAnimationFrame(() => {
				// +1 to skip sentinel row at index 0.
				virtualizerRef.current?.scrollToIndex(
					addedCount + sentinelOffset,
					{
						align: "start",
					},
				);
			});
			return () => cancelAnimationFrame(rafId);
		}
		wasLoadingMoreRef.current = isLoadingMore;
	}, [isLoadingMore, logs.length, sentinelOffset]);

	useEffect(() => {
		if (logsRef) {
			logsRef.current = logs;
		}
	}, [logs, logsRef]);

	const handleScrollChange = useCallback(
		(instance: Virtualizer<HTMLDivElement, Element>) => {
			const isAtBottom =
				(instance.range?.endIndex ?? 0) >= totalCount - 1;
			if (isAtBottom) {
				return setFollow(true);
			}
			if (instance.scrollDirection === "backward") {
				setFollow(false);
				// Load more when the sentinel row comes into view.
				if (
					(instance.range?.startIndex ?? 1) === 0 &&
					hasMore &&
					!isLoadingMore
				) {
					prevLogCountRef.current = logs.length;
					loadMoreHistory();
				}
			}
		},
		[totalCount, logs.length, hasMore, isLoadingMore, loadMoreHistory],
	);

	if (isLoading) {
		return (
			<div className="h-full flex flex-col ">
				<ScrollArea
					className="w-full h-full"
					viewportProps={{ className: "p-2" }}
				>
					{SKELETON_KEYS.map((key) => (
						<Skeleton
							key={key}
							className="w-full h-6 mb-2 last:mb-0"
						/>
					))}
				</ScrollArea>
			</div>
		);
	}

	if (logs.length === 0) {
		if (error) {
			return (
				<div className="h-full flex-1 flex items-center justify-center">
					<div className="max-w-md flex flex-col items-center justify-center flex-1">
						<Icon
							icon={faTriangleExclamation}
							className="text-red-500 mb-2 text-2xl"
						/>
						<div className="text-center">
							<div className="mb-1">Failed to load logs.</div>
							<ErrorDetails error={error} className="text-sm" />
						</div>
					</div>
				</div>
			);
		}
		return (
			<div className="h-full flex flex-1 flex-col items-center justify-center">
				<p>No logs available.</p>
				<p className="text-muted-foreground text-xs mt-1">
					Logs will appear here as they stream in.
				</p>
			</div>
		);
	}

	return (
		<div className="h-full font-mono text-xs text-neutral-100 overflow-hidden flex flex-col">
			{streamError ? (
				<div className="flex items-center gap-2 px-4 py-2 bg-destructive/20 text-destructive-foreground text-xs border-b border-destructive/40 shrink-0">
					<Icon icon={faTriangleExclamation} className="shrink-0" />
					<span>Stream error: {streamError}</span>
				</div>
			) : null}
			<VirtualScrollArea<LogRowData>
				virtualizerRef={virtualizerRef}
				viewportRef={viewportRef}
				onChange={handleScrollChange}
				count={totalCount}
				estimateSize={() => 24}
				className="w-full flex-1 min-h-0"
				scrollerProps={{
					className: "w-full",
				}}
				viewportProps={{}}
				getRowData={(index) => {
					if (hasMore && index === 0) {
						return { isSentinel: true, isLoadingMore };
					}
					return { entry: logs[index - sentinelOffset] };
				}}
				row={LogRow}
			/>
		</div>
	);
}
