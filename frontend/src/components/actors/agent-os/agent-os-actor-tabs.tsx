import {
	faBoxArchive,
	faComments,
	faFolderTree,
	faHardDrive,
	faMicrochip,
	faPlug,
	faTag,
	faWrench,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useLayoutEffect, useRef, useState } from "react";
import {
	cn,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	WithTooltip,
} from "@/components";
import type { ActorId } from "../queries";
import { StatusDot } from "./common";
import { ConnectionsTabConnected } from "./connections-tab";
import { DEFAULT_SESSION_ID } from "./fixtures";
import { FilesystemTabConnected } from "./filesystem-tab";
import { MetadataTabConnected } from "./metadata-tab";
import { MountsTabConnected } from "./mounts-tab";
import { ProcessesTabConnected } from "./processes-tab";
import { SessionRail } from "./session-rail";
import { SoftwareTabConnected } from "./software-tab";
import { ToolsTabConnected } from "./tools-tab";
import { TranscriptTabConnected } from "./transcript-tab";
import {
	AgentOsInspectorProvider,
	useAgentOsInspector,
	useAgentOsManifest,
} from "./use-agent-os-inspector";

const AGENT_OS_TABS = [
	{ id: "transcript", label: "Transcript", icon: faComments },
	{ id: "filesystem", label: "Filesystem", icon: faFolderTree },
	{ id: "processes", label: "Processes", icon: faMicrochip },
	{ id: "tools", label: "Tools", icon: faWrench },
	{ id: "software", label: "Software", icon: faBoxArchive },
	{ id: "mounts", label: "Mounts", icon: faHardDrive },
	{ id: "connections", label: "Connections", icon: faPlug },
	{ id: "metadata", label: "Metadata", icon: faTag },
] as const;

const AGENT_OS_TAB_IDS = AGENT_OS_TABS.map((t) => t.id) as readonly string[];

// Responsive tab labels: hide labels when the trigger row gets narrow.
// Mirrors the default actor inspector shell.
const TAB_LABEL_THRESHOLD = 300;

function useShowTabLabels() {
	const ref = useRef<HTMLDivElement>(null);
	const [showLabels, setShowLabels] = useState(true);

	useLayoutEffect(() => {
		const el = ref.current;
		if (!el) return;
		const observer = new ResizeObserver(() => {
			setShowLabels(el.offsetWidth >= TAB_LABEL_THRESHOLD);
		});
		observer.observe(el);
		return () => observer.disconnect();
	}, []);

	return { ref, showLabels };
}

function AgentOsTabTrigger({
	value,
	label,
	icon,
	showLabels,
}: {
	value: string;
	label: string;
	icon: typeof faComments;
	showLabels: boolean;
}) {
	return (
		<WithTooltip
			delayDuration={0}
			disabled={showLabels}
			content={label}
			trigger={
				<TabsTrigger
					value={value}
					className="text-xs px-2.5 py-1 pb-2 min-w-0 shrink gap-1 isolate before:absolute before:inset-x-0.5 before:top-1 before:bottom-2 before:-z-10 before:rounded-md before:transition-colors hover:before:bg-foreground/[0.06]"
				>
					<Icon icon={icon} className="shrink-0" />
					<span className={showLabels ? "truncate" : "hidden"}>
						{label}
					</span>
				</TabsTrigger>
			}
		/>
	);
}

/**
 * The agentOS actor console. Swaps in for agent-os actors instead of the
 * default `ActorTabsShell`: a left session rail + the agent identity / RUNNING
 * status in the tab row + the 8 agentOS tabs. Each tab body renders
 * `{guardContent || <...>}` so the sleeping/unavailable overlay is preserved.
 */
export function AgentOsActorTabs({
	actorId,
	tab,
	onTabChange,
	className,
	guardContent,
	children,
}: {
	actorId: ActorId;
	tab?: string;
	onTabChange?: (tab: string) => void;
	className?: string;
	guardContent: ReactNode;
	children?: ReactNode;
}) {
	return (
		<AgentOsInspectorProvider>
			<AgentConsole
				actorId={actorId}
				tab={tab}
				onTabChange={onTabChange}
				className={className}
				guardContent={guardContent}
			>
				{children}
			</AgentConsole>
		</AgentOsInspectorProvider>
	);
}

function AgentConsole({
	actorId,
	tab,
	onTabChange,
	className,
	guardContent,
	children,
}: {
	actorId: ActorId;
	tab?: string;
	onTabChange?: (tab: string) => void;
	className?: string;
	guardContent: ReactNode;
	children?: ReactNode;
}) {
	const inspector = useAgentOsInspector();
	const { data: manifest } = useAgentOsManifest(actorId);
	const { data: sessions = [] } = useQuery(
		inspector.sessionsQueryOptions(actorId),
	);
	const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
		DEFAULT_SESSION_ID,
	);
	const { ref: tabListRef, showLabels } = useShowTabLabels();
	const value = tab && AGENT_OS_TAB_IDS.includes(tab) ? tab : "transcript";

	// `guardContent` is null only when the inspector is connectable (running).
	const running = !guardContent;

	return (
		<div className={cn(className, "flex h-full min-h-0 min-w-0 flex-1")}>
			<SessionRail
				sessions={sessions}
				selectedSessionId={selectedSessionId}
				onSelectSession={setSelectedSessionId}
				agentName={manifest?.metadata.actorName}
			/>
			<Tabs
				value={value}
				onValueChange={onTabChange}
				defaultValue={value}
				className="flex min-w-0 flex-1 flex-col min-h-0"
			>
				<div className="flex items-stretch h-[45px] border-b">
					<div
						ref={tabListRef}
						className="flex-1 min-w-0 overflow-hidden h-full"
					>
						<TabsList className="flex border-none h-full items-end min-w-0 overflow-hidden w-full">
							{AGENT_OS_TABS.map((t) => (
								<AgentOsTabTrigger
									key={t.id}
									value={t.id}
									label={t.label}
									icon={t.icon}
									showLabels={showLabels}
								/>
							))}
						</TabsList>
					</div>
					<div className="flex shrink-0 items-center gap-1.5 px-3 text-xs">
						<StatusDot
							color={running ? "green" : "muted"}
							className={running ? "animate-pulse" : undefined}
						/>
						<span
							className={
								running
									? "font-medium text-green-600 dark:text-green-500"
									: "text-muted-foreground"
							}
						>
							{running ? "RUNNING" : "ASLEEP"}
						</span>
					</div>
				</div>

				<TabsContent value="transcript" className="min-h-0 flex-1 mt-0">
					{guardContent || (
						<TranscriptTabConnected
							actorId={actorId}
							selectedSessionId={selectedSessionId}
						/>
					)}
				</TabsContent>
				<TabsContent value="filesystem" className="min-h-0 flex-1 mt-0">
					{guardContent || (
						<FilesystemTabConnected actorId={actorId} />
					)}
				</TabsContent>
				<TabsContent value="processes" className="min-h-0 flex-1 mt-0">
					{guardContent || (
						<ProcessesTabConnected actorId={actorId} />
					)}
				</TabsContent>
				<TabsContent value="tools" className="min-h-0 flex-1 mt-0">
					{guardContent || <ToolsTabConnected actorId={actorId} />}
				</TabsContent>
				<TabsContent value="software" className="min-h-0 flex-1 mt-0">
					{guardContent || <SoftwareTabConnected actorId={actorId} />}
				</TabsContent>
				<TabsContent value="mounts" className="min-h-0 flex-1 mt-0">
					{guardContent || <MountsTabConnected actorId={actorId} />}
				</TabsContent>
				<TabsContent
					value="connections"
					className="min-h-0 flex-1 mt-0"
				>
					{guardContent || (
						<ConnectionsTabConnected actorId={actorId} />
					)}
				</TabsContent>
				<TabsContent
					value="metadata"
					className="min-h-0 flex-1 mt-0 h-full"
				>
					{guardContent || <MetadataTabConnected actorId={actorId} />}
				</TabsContent>
				{children}
			</Tabs>
		</div>
	);
}
