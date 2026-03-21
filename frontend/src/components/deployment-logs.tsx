import type { RivetSse } from "@rivet-gg/cloud";
import { faTriangleExclamation, Icon } from "@rivet-gg/icons";
import type { Virtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useRef, useState } from "react";
import {
	ErrorDetails,
	useCloudNamespaceDataProvider,
} from "@/components/actors";
import { VirtualScrollArea } from "@/components/virtual-scroll-area";
import { AnsiText } from "./lib/ansi";
import { cn } from "./lib/utils";
import { ScrollArea } from "./ui/scroll-area";
import { Skeleton } from "./ui/skeleton";
import { useDeploymentLogsStream } from "./use-deployment-logs-stream";

interface DeploymentLogsProps {
	namespace: string;
	pool: string;
	filter?: string;
	region?: string;
	paused?: boolean;
	logsRef?: React.MutableRefObject<RivetSse.LogStreamEvent[]>;
}

interface LogRowProps {
	className?: string;
	entry: RivetSse.LogStreamEvent.Log;
}

function LogRow({ entry, ...props }: LogRowProps) {
	return (
		<div
			{...props}
			className={cn("font-mono grid grid-cols-subgrid", props.className)}
		>
			<div
				className={cn(
					"grid grid-cols-[max-content,16ch,3fr] gap-3 whitespace-pre-wrap break-words px-4 py-1 border-b",
					{
						"text-red-400": entry.data.stream === "stderr",
						"text-muted-foreground": entry.data.stream !== "stderr",
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
	namespace,
	pool,
	filter,
	region,
	paused,
	logsRef,
}: DeploymentLogsProps) {
	const { project } = useCloudNamespaceDataProvider();
	const { logs, isLoading, error } = useDeploymentLogsStream({
		project,
		namespace,
		pool,
		filter,
		region,
		paused,
	});

	const viewportRef = useRef<HTMLDivElement>(null);
	const virtualizerRef = useRef<Virtualizer<HTMLDivElement, Element>>(null);
	const [follow, setFollow] = useState(true);

	useEffect(() => {
		if (follow && !isLoading && virtualizerRef.current && logs.length > 0) {
			// https://github.com/TanStack/virtual/issues/537
			const rafId = requestAnimationFrame(() => {
				virtualizerRef.current?.scrollToIndex(logs.length - 1, {
					align: "end",
				});
			});
			return () => cancelAnimationFrame(rafId);
		}
	}, [logs.length, follow, isLoading]);

	useEffect(() => {
		if (logsRef) {
			logsRef.current = logs;
		}
	}, [logs, logsRef]);

	const handleScrollChange = useCallback(
		(instance: Virtualizer<HTMLDivElement, Element>) => {
			const isAtBottom =
				(instance.range?.endIndex ?? 0) >= logs.length - 1;
			if (isAtBottom) {
				return setFollow(true);
			}
			if (instance.scrollDirection === "backward") {
				return setFollow(false);
			}
		},
		[logs.length],
	);

	if (isLoading) {
		return (
			<div className="h-full flex flex-col ">
				<ScrollArea
					className="w-full h-full"
					viewportProps={{ className: "p-2" }}
				>
					{Array.from({ length: 40 }).map((_, i) => (
						<Skeleton
							key={i}
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
		<div className=" h-full font-mono text-xs text-neutral-100 overflow-hidden">
			<VirtualScrollArea<{ index: number }>
				virtualizerRef={virtualizerRef}
				viewportRef={viewportRef}
				onChange={handleScrollChange}
				count={logs.length}
				estimateSize={() => 24}
				className="w-full h-full"
				scrollerProps={{
					className: "w-full",
				}}
				viewportProps={{}}
				getRowData={(index) => ({
					index: index,
				})}
				row={(props) => <LogRow {...props} entry={logs[props.index]} />}
			/>
		</div>
	);
}
