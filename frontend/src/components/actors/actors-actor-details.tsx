import { faQuestionSquare, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { memo, type ReactNode, Suspense } from "react";
import {
	cn,
	Flex,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@/components";
import { ActorConfigTab } from "./actor-config-tab";
import { ActorConnectionsTab } from "./actor-connections-tab";
import { ActorDatabaseTab } from "./actor-db-tab";
import { ActorDetailsSettingsProvider } from "./actor-details-settings";
import { useActorInspector } from "./actor-inspector-context";
import { ActorLogsTab } from "./actor-logs-tab";
import { ActorQueueTab } from "./actor-queue-tab";
import { ActorStateTab } from "./actor-state-tab";
import { QueriedActorStatus } from "./actor-status";
import { ActorStopButton } from "./actor-stop-button";
import { useActorsView } from "./actors-view-context-provider";
import { ActorConsole } from "./console/actor-console";
import {
	GuardConnectableInspector,
	useInspectorGuard,
} from "./guard-connectable-inspector";
import type { ActorId } from "./queries";
import { ActorWorkerContextProvider } from "./worker/actor-worker-context";
import { ActorWorkflowTab } from "./workflow/actor-workflow-tab";

interface ActorsActorDetailsProps {
	tab?: string;
	actorId: ActorId;
	onTabChange?: (tab: string) => void;
	onExportLogs?: (
		actorId: string,
		typeFilter?: string,
		filter?: string,
	) => Promise<void>;
	isExportingLogs?: boolean;
}

export const ActorsActorDetails = memo(
	({ tab, onTabChange, actorId }: ActorsActorDetailsProps) => {
		return (
			<GuardConnectableInspector actorId={actorId}>
				<ActorDetailsSettingsProvider>
					<div className="flex flex-col h-full flex-1">
						<ActorTabs
							actorId={actorId}
							tab={tab}
							onTabChange={onTabChange}
						/>

						<Console actorId={actorId} />
					</div>
				</ActorDetailsSettingsProvider>
			</GuardConnectableInspector>
		);
	},
);

function Console({ actorId }: { actorId: ActorId }) {
	const guardContent = useInspectorGuard();

	if (guardContent) return null;

	return (
		<ActorWorkerContextProvider actorId={actorId}>
			<ActorConsole actorId={actorId} />
		</ActorWorkerContextProvider>
	);
}

export const ActorsActorEmptyDetails = () => {
	const { copy } = useActorsView();
	return (
		<div className="flex flex-col h-full w-full min-w-0 min-h-0 flex-1">
			<ActorTabs disabled>
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col gap-2">
					<Icon icon={faQuestionSquare} className="text-4xl" />
					<p className="max-w-[400px]">{copy.selectActor}</p>
				</div>
			</ActorTabs>
		</div>
	);
};

const TAB_PRIORITY = [
	"workflow",
	"database",
	"state",
	"queue",
	"connections",
	"metadata",
] as const;

type TabId = (typeof TAB_PRIORITY)[number];

function useActorTabVisibility(actorId: ActorId) {
	const inspector = useActorInspector();

	const { data: stateData } = useQuery(
		inspector.actorStateQueryOptions(actorId),
	);
	const { data: isDatabaseEnabled } = useQuery(
		inspector.actorDatabaseEnabledQueryOptions(actorId),
	);
	const { data: isWorkflowEnabled } = useQuery(
		inspector.actorIsWorkflowEnabledQueryOptions(actorId),
	);

	const isStateEnabled = stateData?.isEnabled ?? false;
	const isQueueEnabled = inspector.features.queue.supported;

	const hiddenTabs = new Set<TabId>();
	if (!isWorkflowEnabled) hiddenTabs.add("workflow");
	if (!isDatabaseEnabled) hiddenTabs.add("database");
	if (!isStateEnabled) hiddenTabs.add("state");
	if (!isQueueEnabled) hiddenTabs.add("queue");

	const firstAvailableTab =
		TAB_PRIORITY.find((tab) => !hiddenTabs.has(tab)) ?? "connections";

	return { hiddenTabs, firstAvailableTab };
}

function ActorTabTriggers({
	actorId,
	tab,
	onTabChange,
	className,
	children,
}: {
	actorId: ActorId;
	tab?: string;
	onTabChange?: (tab: string) => void;
	className?: string;
	children?: ReactNode;
}) {
	const { hiddenTabs, firstAvailableTab } = useActorTabVisibility(actorId);
	const guardContent = useInspectorGuard();

	const normalizedTab = tab === "events" ? "traces" : tab;
	const isTabHidden = normalizedTab && hiddenTabs.has(normalizedTab as TabId);
	const value =
		!normalizedTab || isTabHidden ? firstAvailableTab : normalizedTab;

	return (
		<ActorTabsShell
			actorId={actorId}
			value={value}
			onValueChange={onTabChange}
			className={className}
			guardContent={guardContent}
			hiddenTabs={hiddenTabs}
		>
			{children}
		</ActorTabsShell>
	);
}

function ActorTabsShell({
	actorId,
	value,
	onValueChange,
	className,
	guardContent,
	hiddenTabs,
	children,
}: {
	actorId: ActorId;
	value: string;
	onValueChange?: (tab: string) => void;
	className?: string;
	guardContent: ReactNode;
	hiddenTabs: Set<TabId>;
	children?: ReactNode;
}) {
	return (
		<Tabs
			value={value}
			onValueChange={onValueChange}
			defaultValue={value}
			className={cn(className, "flex-1 min-h-0 min-w-0 flex flex-col ")}
		>
			<div className="flex justify-between items-center border-b h-[45px]">
				<div className="flex flex-1 items-center h-full w-full ">
					<TabsList className="overflow-auto border-none h-full items-end">
						{!hiddenTabs.has("workflow") && (
							<TabsTrigger
								value="workflow"
								className="text-xs px-3 py-1 pb-2"
							>
								Workflow
							</TabsTrigger>
						)}
						{!hiddenTabs.has("database") && (
							<TabsTrigger
								value="database"
								className="text-xs px-3 py-1 pb-2"
							>
								Database
							</TabsTrigger>
						)}
						{!hiddenTabs.has("state") && (
							<TabsTrigger
								value="state"
								className="text-xs px-3 py-1 pb-2"
							>
								State
							</TabsTrigger>
						)}
						{!hiddenTabs.has("queue") && (
							<TabsTrigger
								value="queue"
								className="text-xs px-3 py-1 pb-2"
							>
								Queue
							</TabsTrigger>
						)}
						<TabsTrigger
							value="connections"
							className="text-xs px-3 py-1 pb-2"
						>
							Connections
						</TabsTrigger>
						<TabsTrigger
							value="metadata"
							className="text-xs px-3 py-1 pb-2"
						>
							Metadata
						</TabsTrigger>
					</TabsList>
					<Flex
						gap="2"
						justify="between"
						items="center"
						className="h-[36px] pb-3 pt-2 pr-4"
					>
						<QueriedActorStatus
							className="text-sm h-auto"
							actorId={actorId}
						/>
						<ActorStopButton actorId={actorId} />
					</Flex>
				</div>
			</div>
			<TabsContent value="logs" className="min-h-0 flex-1 mt-0 h-full">
				<Suspense fallback={<ActorLogsTab.Skeleton />}>
					{guardContent || <ActorLogsTab actorId={actorId} />}
				</Suspense>
			</TabsContent>
			<TabsContent
				value="metadata"
				className="min-h-0 flex-1 mt-0 h-full"
			>
				<ActorConfigTab actorId={actorId} />
			</TabsContent>
			<TabsContent value="connections" className="min-h-0 flex-1 mt-0">
				{guardContent || <ActorConnectionsTab actorId={actorId} />}
			</TabsContent>
			<TabsContent value="queue" className="min-h-0 flex-1 mt-0">
				{guardContent || <ActorQueueTab actorId={actorId} />}
			</TabsContent>
			<TabsContent
				value="workflow"
				className="min-h-0 flex-1 mt-0 h-full"
			>
				{guardContent || <ActorWorkflowTab actorId={actorId} />}
			</TabsContent>
			<TabsContent
				value="database"
				className="min-h-0 min-w-0 flex-1 mt-0 h-full"
			>
				{guardContent || <ActorDatabaseTab actorId={actorId} />}
			</TabsContent>
			<TabsContent value="state" className="min-h-0 flex-1 mt-0 relative">
				{guardContent || <ActorStateTab actorId={actorId} />}
			</TabsContent>
			{children}
		</Tabs>
	);
}

function ActorTabsNotRunning({
	actorId,
	tab,
	onTabChange,
	className,
	children,
}: {
	actorId: ActorId;
	tab?: string;
	onTabChange?: (tab: string) => void;
	className?: string;
	children?: ReactNode;
}) {
	const guardContent = useInspectorGuard();
	const normalizedTab = tab === "events" ? "traces" : tab;
	const value = normalizedTab || "connections";

	return (
		<ActorTabsShell
			actorId={actorId}
			value={value}
			onValueChange={onTabChange}
			className={className}
			guardContent={guardContent}
			hiddenTabs={new Set()}
		>
			{children}
		</ActorTabsShell>
	);
}

function ActorTabsWithId({
	actorId,
	tab,
	onTabChange,
	className,
	children,
}: {
	actorId: ActorId;
	tab?: string;
	onTabChange?: (tab: string) => void;
	className?: string;
	children?: ReactNode;
}) {
	const guardContent = useInspectorGuard();

	// Inspector context is only available when the actor is running (guardContent is null).
	// We must not call useActorInspector hooks when the inspector is unavailable.
	if (guardContent) {
		return (
			<ActorTabsNotRunning
				actorId={actorId}
				tab={tab}
				onTabChange={onTabChange}
				className={className}
			>
				{children}
			</ActorTabsNotRunning>
		);
	}

	return (
		<ActorTabTriggers
			actorId={actorId}
			tab={tab}
			onTabChange={onTabChange}
			className={className}
		>
			{children}
		</ActorTabTriggers>
	);
}

export function ActorTabs({
	tab,
	onTabChange,
	actorId,
	className,
	disabled,
	children,
}: {
	disabled?: boolean;
	tab?: string;
	onTabChange?: (tab: string) => void;
	actorId?: ActorId;
	className?: string;
	children?: ReactNode;
}) {
	if (actorId) {
		return (
			<ActorTabsWithId
				actorId={actorId}
				tab={tab}
				onTabChange={onTabChange}
				className={className}
			>
				{children}
			</ActorTabsWithId>
		);
	}

	return (
		<Tabs
			value={undefined}
			className={cn(className, "flex-1 min-h-0 min-w-0 flex flex-col ")}
		>
			<div className="flex justify-between items-center border-b h-[45px]">
				<div className="flex flex-1 items-center h-full w-full ">
					<TabsList className="overflow-auto border-none h-full items-end">
						<TabsTrigger
							disabled={disabled}
							value="workflow"
							className="text-xs px-3 py-1 pb-2"
						>
							Workflow
						</TabsTrigger>
						<TabsTrigger
							disabled={disabled}
							value="database"
							className="text-xs px-3 py-1 pb-2"
						>
							Database
						</TabsTrigger>
						<TabsTrigger
							disabled={disabled}
							value="state"
							className="text-xs px-3 py-1 pb-2"
						>
							State
						</TabsTrigger>
						<TabsTrigger
							disabled={disabled}
							value="queue"
							className="text-xs px-3 py-1 pb-2"
						>
							Queue
						</TabsTrigger>
						<TabsTrigger
							disabled={disabled}
							value="connections"
							className="text-xs px-3 py-1 pb-2"
						>
							Connections
						</TabsTrigger>
						<TabsTrigger
							disabled={disabled}
							value="metadata"
							className="text-xs px-3 py-1 pb-2"
						>
							Metadata
						</TabsTrigger>
					</TabsList>
				</div>
			</div>
			{children}
		</Tabs>
	);
}
