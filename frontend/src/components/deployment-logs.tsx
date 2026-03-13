import type { RivetSse } from "@rivet-gg/cloud";
import { faTriangleExclamation, Icon } from "@rivet-gg/icons";
import type { Virtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useRef, useState } from "react";
import {
	ErrorDetails,
	useCloudNamespaceDataProvider,
} from "@/components/actors";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { VirtualScrollArea } from "@/components/virtual-scroll-area";
import { ScrollArea } from "./ui/scroll-area";
import { Skeleton } from "./ui/skeleton";
import { useDeploymentLogsStream } from "./use-deployment-logs-stream";

interface DeploymentLogsProps {
	namespace: string;
	pool: string;
	filter?: string;
	region?: string;
	paused?: boolean;
	onLogsChange?: (logs: RivetSse.LogEntry[]) => void;
}

interface LogRowProps {
	"data-index": number;
	className?: string;
	entry: RivetSse.LogEntry;
	isNew: boolean;
}

function LogRow({ entry, isNew }: LogRowProps) {
	return (
		<div
			className={`animate-in fade-in duration-300 ${isNew ? "" : "opacity-100"} font-mono`}
			style={{
				animationDelay: isNew ? "0ms" : undefined,
			}}
		>
			<div
				className={`flex gap-3 whitespace-pre-wrap break-words px-4 py-1 ${
					entry.stream === "stderr"
						? "text-red-400"
						: "text-green-400"
				}`}
			>
				<span className="text-neutral-500 shrink-0">
					{entry.timestamp}
				</span>
				{entry.region ? (
					<span className="text-neutral-600 shrink-0">
						[{entry.region}]
					</span>
				) : null}
				<span className="flex-1">{entry.message}</span>
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
	onLogsChange,
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
	const prevLengthRef = useRef(logs.length);
	const [follow, setFollow] = useState(true);

	useEffect(() => {
		onLogsChange?.(logs);
	}, [logs, onLogsChange]);

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
		prevLengthRef.current = logs.length;
	}, [logs.length, follow, isLoading]);

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

	const isNewLog = useCallback((index: number) => {
		return index >= prevLengthRef.current;
	}, []);

	if (isLoading) {
		return (
			<div className="h-full flex flex-col p-2 ">
				<ScrollArea className="w-full h-full">
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
			<VirtualScrollArea
				virtualizerRef={virtualizerRef}
				viewportRef={viewportRef}
				onChange={handleScrollChange}
				count={logs.length}
				estimateSize={() => 28}
				className="w-full h-full"
				scrollerProps={{
					className: "w-full",
				}}
				row={(props: LogRowProps) => (
					<LogRow
						{...props}
						entry={logs[props["data-index"]]}
						isNew={isNewLog(props["data-index"])}
					/>
				)}
				getRowData={(index) => ({
					"data-index": index,
				})}
			/>
		</div>
	);
}
