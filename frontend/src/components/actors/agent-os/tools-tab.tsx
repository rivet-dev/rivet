import { faChevronRight, faCircleExclamation, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
	cn,
	RelativeTime,
	ScrollArea,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	WithTooltip,
} from "@/components";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";
import type { ActorId } from "../queries";
import { AgentOsEmpty } from "./common";
import type { Invocation, Toolkit, ToolsData } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

function compact(value: unknown): string {
	if (value == null) return "—";
	if (typeof value === "string") return value;
	try {
		return JSON.stringify(value);
	} catch {
		return String(value);
	}
}

export function ToolsTab({ toolkits, invocations }: ToolsData) {
	return (
		<ScrollArea className="h-full w-full">
			<section className="px-4 py-4">
				<header className="mb-3">
					<h3 className="text-sm font-semibold">
						Registered toolkits
					</h3>
					<p className="mt-0.5 text-xs text-muted-foreground">
						Host tools exposed to the agent as CLI commands inside
						the VM.
					</p>
				</header>
				{toolkits.length === 0 ? (
					<p className="py-4 text-sm text-muted-foreground">
						No toolkits registered.
					</p>
				) : (
					<div className="space-y-2">
						{toolkits.map((toolkit) => (
							<ToolkitGroup
								key={toolkit.name}
								toolkit={toolkit}
							/>
						))}
					</div>
				)}
			</section>

			<section className="border-t px-4 py-4">
				<header className="mb-3">
					<h3 className="text-sm font-semibold">
						Recent invocations
					</h3>
					<p className="mt-0.5 text-xs text-muted-foreground">
						Last {invocations.length} tool calls across all
						sessions.
					</p>
				</header>
				{invocations.length === 0 ? (
					<p className="py-2 text-sm text-muted-foreground">
						No invocations yet.
					</p>
				) : (
					<InvocationsTable invocations={invocations} />
				)}
			</section>
		</ScrollArea>
	);
}

function ToolkitGroup({ toolkit }: { toolkit: Toolkit }) {
	const [open, setOpen] = useState(true);
	return (
		<Collapsible
			open={open}
			onOpenChange={setOpen}
			className="rounded-md border"
		>
			<CollapsibleTrigger className="flex w-full items-center gap-2 px-3 py-2 text-left">
				<Icon
					icon={faChevronRight}
					className={cn(
						"size-3 shrink-0 text-muted-foreground transition-transform",
						open && "rotate-90",
					)}
				/>
				<span className="font-mono text-sm font-medium">
					{toolkit.name}
				</span>
				<span className="text-xs text-muted-foreground">
					{toolkit.tools.length} tool
					{toolkit.tools.length === 1 ? "" : "s"}
				</span>
			</CollapsibleTrigger>
			<CollapsibleContent>
				<div className="divide-y border-t">
					{toolkit.tools.map((tool) => (
						<div key={tool.tool} className="px-3 py-2.5 pl-8">
							<div className="flex items-baseline gap-2">
								<span className="font-mono text-sm font-medium">
									{toolkit.name}.{tool.tool}
								</span>
								<code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
									{tool.command}
								</code>
							</div>
							<p className="mt-1 text-xs text-muted-foreground">
								{tool.description}
							</p>
							{tool.args.length > 0 ? (
								<div className="mt-1.5 flex flex-wrap gap-1.5">
									{tool.args.map((arg) => (
										<span
											key={arg.name}
											className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs"
										>
											<span className="font-mono">
												{arg.name}
											</span>
											<span className="text-muted-foreground">
												{arg.type}
												{arg.optional ? "?" : ""}
											</span>
										</span>
									))}
								</div>
							) : null}
						</div>
					))}
				</div>
			</CollapsibleContent>
		</Collapsible>
	);
}

function InvocationsTable({ invocations }: { invocations: Invocation[] }) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Tool</TableHead>
					<TableHead>Input</TableHead>
					<TableHead>Output</TableHead>
					<TableHead className="text-right">Latency</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{invocations.map((inv) => (
					<TableRow key={`${inv.tool}-${inv.at}-${inv.latencyMs}`}>
						<TableCell className="py-2">
							<span className="inline-flex items-center gap-1.5 font-mono text-xs">
								{inv.error ? (
									<WithTooltip
										content={inv.error}
										trigger={
											<Icon
												icon={faCircleExclamation}
												className="size-3 text-red-500"
											/>
										}
									/>
								) : null}
								{inv.tool}
							</span>
						</TableCell>
						<TableCell className="max-w-[220px] truncate py-2 font-mono text-xs text-muted-foreground">
							{compact(inv.input)}
						</TableCell>
						<TableCell
							className={cn(
								"max-w-[260px] truncate py-2 font-mono text-xs",
								inv.error
									? "text-red-500"
									: "text-muted-foreground",
							)}
						>
							{inv.error ?? compact(inv.output)}
						</TableCell>
						<TableCell className="whitespace-nowrap py-2 text-right text-xs tabular-nums text-muted-foreground">
							{inv.latencyMs} ms
							<span className="mx-1">·</span>
							<RelativeTime time={new Date(inv.at)} />
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	);
}

export function ToolsTabConnected({ actorId }: { actorId: ActorId }) {
	const inspector = useAgentOsInspector();
	const { data } = useQuery(inspector.toolsQueryOptions(actorId));
	if (!data) return <AgentOsEmpty>No tools available.</AgentOsEmpty>;
	return <ToolsTab toolkits={data.toolkits} invocations={data.invocations} />;
}
