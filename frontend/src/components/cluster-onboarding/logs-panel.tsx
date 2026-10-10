import type { Rivet } from "@rivet-gg/cloud";
import { faArrowDown, faSearch, Icon } from "@rivet-gg/icons";
import { useEffect, useMemo, useRef, useState } from "react";
import { DeploymentLogsExportMenu } from "@/components/deployment-logs";
import { AnsiText } from "@/components/lib/ansi";
import { cn } from "@/components/lib/utils";
import { Input } from "@/components/ui/input";
import { useLogScroll } from "@/components/use-log-scroll";
import { VirtualScrollArea } from "@/components/virtual-scroll-area";
import {
	type LogEntry,
	type MockActor,
	mockActorLogs,
	mockActors,
	mockLiveLogLine,
} from "./actor-logs";
import type { UseCase } from "./catalog";

interface LogsPanelProps {
	useCase: UseCase;
	region: string;
	/** Used for the export filename. */
	clusterName: string;
	className?: string;
}

/** A log line plus the actor that wrote it, for the actor column. */
type ClusterLog = LogEntry & { actor: MockActor };

/**
 * One log stream for the whole cluster: every actor's lines merged by
 * time, with a text filter. Rows mirror `DeploymentLogs`; only the data is
 * generated.
 */
export function LogsPanel({
	useCase,
	region,
	clusterName,
	className,
}: LogsPanelProps) {
	const actors = useMemo(
		() => mockActors(useCase, region),
		[useCase, region],
	);
	const [logs, setLogs] = useState<ClusterLog[]>(() =>
		actors
			.flatMap((actor) =>
				mockActorLogs(actor).map((entry) => ({ ...entry, actor })),
			)
			.sort((a, b) => a.data.timestamp.localeCompare(b.data.timestamp)),
	);
	const [filter, setFilter] = useState("");
	const logsRef = useRef<Rivet.LogStreamEvent.Log[]>([]);

	const running = useMemo(
		() => actors.filter((a) => a.status === "running"),
		[actors],
	);
	useEffect(() => {
		if (running.length === 0) return;
		let seq = 0;
		const id = window.setInterval(() => {
			seq += 1;
			const actor = running[seq % running.length];
			const entry = mockLiveLogLine(actor, seq);
			setLogs((prev) => [...prev.slice(-2000), { ...entry, actor }]);
		}, 1200);
		return () => window.clearInterval(id);
	}, [running]);

	const visible = useMemo(() => {
		const q = filter.trim().toLowerCase();
		if (!q) return logs;
		return logs.filter(
			({ data, actor }) =>
				data.message.toLowerCase().includes(q) ||
				actor.name.toLowerCase().includes(q) ||
				actor.key.toLowerCase().includes(q) ||
				actor.id.startsWith(q),
		);
	}, [logs, filter]);

	useEffect(() => {
		logsRef.current = visible;
	}, [visible]);

	const {
		displayedLogs,
		follow,
		setFollow,
		viewportRef,
		virtualizerRef,
		handleScrollChange,
		totalCount,
	} = useLogScroll({
		logs: visible,
		hasMore: false,
		isLoading: false,
		isLoadingMore: false,
		loadMoreHistory: () => {},
	});

	return (
		<div
			className={cn(
				"my-2 mr-2 flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border bg-card",
				className,
			)}
		>
			<div className="flex items-center gap-2 border-b px-3 py-2">
				<div className="relative flex-1">
					<Icon
						icon={faSearch}
						className="pointer-events-none absolute left-2.5 top-1/2 size-3 -translate-y-1/2 text-muted-foreground"
					/>
					<Input
						value={filter}
						onChange={(e) => setFilter(e.target.value)}
						placeholder="Filter logs by text, actor, key, or ID"
						className="h-7 pl-7 text-xs"
						aria-label="Filter logs"
					/>
				</div>
				<span className="shrink-0 text-xs tabular-nums text-muted-foreground">
					{visible.length.toLocaleString()} lines · {running.length}{" "}
					actors live
				</span>
				<DeploymentLogsExportMenu
					logsRef={logsRef}
					filename={`${clusterName}-logs.txt`}
					className="h-6 text-xs"
				/>
			</div>
			<div className="relative min-h-0 flex-1 font-mono text-xs">
				<VirtualScrollArea<{ entry: ClusterLog }>
					virtualizerRef={virtualizerRef}
					viewportRef={viewportRef}
					onChange={handleScrollChange}
					count={totalCount}
					estimateSize={() => 24}
					type="auto"
					className="h-full w-full"
					scrollerProps={{ className: "w-full" }}
					viewportProps={{}}
					getRowData={(index) => ({
						// The hook only ever hands back the objects it was
						// given, so the actor field survives the cast.
						entry: displayedLogs[index] as ClusterLog,
					})}
					row={LogRow}
				/>
				{!follow ? (
					<div className="absolute bottom-4 left-1/2 z-10 -translate-x-1/2">
						<button
							type="button"
							className="flex items-center gap-1.5 rounded-full bg-primary px-3 py-1.5 font-sans text-xs font-medium text-primary-foreground shadow-lg transition-colors hover:bg-primary/90"
							onClick={() => {
								setFollow(true);
								virtualizerRef.current?.scrollToIndex(
									totalCount - 1,
									{ align: "end" },
								);
							}}
						>
							<Icon icon={faArrowDown} className="size-3" />
							Back to newest
						</button>
					</div>
				) : null}
			</div>
		</div>
	);
}

function LogRow({
	entry,
	className,
	...props
}: {
	entry?: ClusterLog;
	className?: string;
}) {
	if (!entry) return null;
	const { data, actor } = entry;
	return (
		<div
			{...props}
			className={cn("grid grid-cols-subgrid font-mono", className)}
		>
			<div
				className={cn(
					"grid gap-3 whitespace-pre-wrap break-words border-b px-4 py-1",
					data.severity === "error"
						? "text-red-400"
						: data.severity === "warn"
							? "text-amber-500 dark:text-amber-400"
							: "text-muted-foreground",
				)}
				style={{
					gridTemplateColumns: "max-content 22ch 3fr",
				}}
			>
				<span className="shrink-0 select-none text-neutral-500">
					{data.timestamp}
				</span>
				<span className="truncate text-neutral-600">
					{actor.name}{" "}
					<span className="text-neutral-500">
						{actor.id.split("-")[0]}
					</span>
				</span>
				<span className="flex-1">
					<AnsiText text={data.message} />
				</span>
			</div>
		</div>
	);
}
