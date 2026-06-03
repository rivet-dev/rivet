import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
	Badge,
	RelativeTime,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";
import type { ActorId } from "../queries";
import { AgentOsEmpty, formatBytes, StatusDot } from "./common";
import { DEFAULT_PID } from "./fixtures";
import type { ProcessInfo, ProcessStatus } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

const STATUS_DOT = {
	running: "green",
	sleeping: "muted",
	exited: "red",
} as const satisfies Record<ProcessStatus, "green" | "muted" | "red">;

export function ProcessesTab({
	processes,
	defaultPid,
}: {
	processes: ProcessInfo[];
	defaultPid?: number;
}) {
	const [selectedPid, setSelectedPid] = useState<number | undefined>(
		defaultPid ?? processes[0]?.pid,
	);
	const selected = processes.find((p) => p.pid === selectedPid);

	if (processes.length === 0) {
		return <AgentOsEmpty>No processes running.</AgentOsEmpty>;
	}

	return (
		<ResizablePanelGroup direction="horizontal" className="h-full">
			<ResizablePanel defaultSize={64} minSize={40}>
				<ScrollArea className="h-full w-full">
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>PID</TableHead>
								<TableHead>PPID</TableHead>
								<TableHead>Command</TableHead>
								<TableHead>Started</TableHead>
								<TableHead className="text-right">
									CPU
								</TableHead>
								<TableHead className="text-right">
									Mem
								</TableHead>
								<TableHead>Status</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{processes.map((proc) => (
								<TableRow
									key={proc.pid}
									isClickable
									data-state={
										proc.pid === selectedPid
											? "selected"
											: undefined
									}
									onClick={() => setSelectedPid(proc.pid)}
								>
									<TableCell className="py-2 font-mono">
										{proc.pid}
									</TableCell>
									<TableCell className="py-2 font-mono text-muted-foreground">
										{proc.ppid}
									</TableCell>
									<TableCell className="py-2 font-mono text-xs">
										{proc.command}
									</TableCell>
									<TableCell className="py-2 text-muted-foreground">
										<RelativeTime
											time={new Date(proc.startedAt)}
										/>
									</TableCell>
									<TableCell className="py-2 text-right tabular-nums text-muted-foreground">
										{proc.cpu != null
											? `${proc.cpu.toFixed(1)}%`
											: "—"}
									</TableCell>
									<TableCell className="py-2 text-right tabular-nums text-muted-foreground">
										{formatBytes(proc.memBytes)}
									</TableCell>
									<TableCell className="py-2">
										<span className="inline-flex items-center gap-1.5 text-xs">
											<StatusDot
												color={STATUS_DOT[proc.status]}
											/>
											{proc.status}
										</span>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</ScrollArea>
			</ResizablePanel>
			<ResizableHandle />
			<ResizablePanel defaultSize={36} minSize={24}>
				{selected ? (
					<ProcessDetail process={selected} />
				) : (
					<AgentOsEmpty>Select a process.</AgentOsEmpty>
				)}
			</ResizablePanel>
		</ResizablePanelGroup>
	);
}

function ProcessDetail({ process }: { process: ProcessInfo }) {
	return (
		<div className="flex h-full flex-col">
			<div className="flex items-center gap-2 border-b px-4 py-3">
				<span className="font-mono text-sm">pid {process.pid}</span>
				<span className="truncate font-mono text-xs text-muted-foreground">
					{process.command}
				</span>
				{process.signal ? (
					<Badge variant="destructive-muted" className="ml-auto">
						{process.signal}
					</Badge>
				) : null}
			</div>
			<div className="px-4 py-2 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
				stdout (tail)
			</div>
			<ScrollArea className="min-h-0 flex-1">
				<pre className="whitespace-pre-wrap break-words px-4 pb-4 font-mono text-xs leading-relaxed">
					{process.stdoutTail ?? "(no output captured)"}
				</pre>
			</ScrollArea>
		</div>
	);
}

export function ProcessesTabConnected({ actorId }: { actorId: ActorId }) {
	const inspector = useAgentOsInspector();
	const { data = [] } = useQuery(inspector.processesQueryOptions(actorId));
	return <ProcessesTab processes={data} defaultPid={DEFAULT_PID} />;
}
