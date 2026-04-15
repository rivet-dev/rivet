import {
	faCubesStacked,
	faDatabase,
	faDiagramProject,
	faInbox,
	faLogs,
	faPlug,
	faQuestionSquare,
	faTag,
	faTerminal,
	Icon,
} from "@rivet-gg/icons";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import {
	memo,
	type ReactNode,
	Suspense,
	useLayoutEffect,
	useRef,
	useState,
} from "react";
import {
	cn,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	WithTooltip,
} from "@/components";
import { DeploymentLogs } from "@/components/deployment-logs";
import { ActorConfigTab } from "./actor-config-tab";
import { ActorConnectionsTab } from "./actor-connections-tab";
import { ActorDatabaseTab } from "./actor-db-tab";
import { ActorDetailsSettingsProvider } from "./actor-details-settings";
import { useActorInspector } from "./actor-inspector-context";
import { ActorLogsTab } from "./actor-logs-tab";
import { ActorQueueTab } from "./actor-queue-tab";
import { ActorStateTab } from "./actor-state-tab";
import { QueriedActorStatusIndicator } from "./actor-status-indicator";
import { useActorsView } from "./actors-view-context-provider";
import { ActorConsoleFull } from "./console/actor-console";
import { useCloudNamespaceDataProvider } from "./data-provider";
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
					</div>
				</ActorDetailsSettingsProvider>
			</GuardConnectableInspector>
		);
	},
);

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
	"deployment-logs",
	"console",
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

	const provider = useCloudNamespaceDataProvider();
	const { data: hasManagedPool } = useSuspenseQuery(
		provider.currentNamespaceHasManagedPoolQueryOptions(),
	);

	const hiddenTabs = new Set<TabId>();
	if (!isWorkflowEnabled) hiddenTabs.add("workflow");
	if (!isDatabaseEnabled) hiddenTabs.add("database");
	if (!isStateEnabled) hiddenTabs.add("state");
	if (!isQueueEnabled) hiddenTabs.add("queue");
	if (__APP_TYPE__ !== "cloud" || !hasManagedPool)
		hiddenTabs.add("deployment-logs");

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

const TAB_LABEL_THRESHOLD = 300; /* in px */

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
	const { ref: tabListRef, showLabels } = useShowTabLabels();

	return (
		<Tabs
			value={value}
			onValueChange={onValueChange}
			defaultValue={value}
			className={cn(className, "flex-1 min-h-0 min-w-0 flex flex-col ")}
		>
			<div className="flex justify-between items-center border-b h-[45px]">
				<div className="flex flex-1 items-center h-full w-full min-w-0">
					<div
						ref={tabListRef}
						className="flex-1 min-w-0 overflow-hidden h-full"
					>
						<TabsList className="flex border-none h-full items-end min-w-0 overflow-hidden w-full">
							{!hiddenTabs.has("workflow") && (
								<WithTooltip
									delayDuration={0}
									disabled={showLabels}
									trigger={
										<TabsTrigger
											value="workflow"
											className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
										>
											<Icon
												icon={faDiagramProject}
												className="shrink-0"
											/>
											<span
												className={
													showLabels
														? "truncate"
														: "hidden"
												}
											>
												Workflow
											</span>
										</TabsTrigger>
									}
									content="Workflow"
								/>
							)}
							{!hiddenTabs.has("database") && (
								<WithTooltip
									delayDuration={0}
									disabled={showLabels}
									trigger={
										<TabsTrigger
											value="database"
											className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
										>
											<Icon
												icon={faDatabase}
												className="shrink-0"
											/>
											<span
												className={
													showLabels
														? "truncate"
														: "hidden"
												}
											>
												Database
											</span>
										</TabsTrigger>
									}
									content="Database"
								/>
							)}
							{!hiddenTabs.has("state") && (
								<WithTooltip
									delayDuration={0}
									disabled={showLabels}
									trigger={
										<TabsTrigger
											value="state"
											className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
										>
											<Icon
												icon={faCubesStacked}
												className="shrink-0"
											/>
											<span
												className={
													showLabels
														? "truncate"
														: "hidden"
												}
											>
												State
											</span>
										</TabsTrigger>
									}
									content="State"
								/>
							)}
							{!hiddenTabs.has("queue") && (
								<WithTooltip
									delayDuration={0}
									disabled={showLabels}
									trigger={
										<TabsTrigger
											value="queue"
											className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
										>
											<Icon
												icon={faInbox}
												className="shrink-0"
											/>
											<span
												className={
													showLabels
														? "truncate"
														: "hidden"
												}
											>
												Queue
											</span>
										</TabsTrigger>
									}
									content="Queue"
								/>
							)}
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										value="connections"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faPlug}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Connections
										</span>
									</TabsTrigger>
								}
								content="Connections"
							/>

							{!hiddenTabs.has("deployment-logs") && (
								<WithTooltip
									delayDuration={0}
									disabled={showLabels}
									trigger={
										<TabsTrigger
											value="deployment-logs"
											className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
										>
											<Icon
												icon={faLogs}
												className="shrink-0"
											/>
											<span
												className={
													showLabels
														? "truncate"
														: "hidden"
												}
											>
												Logs
											</span>
										</TabsTrigger>
									}
									content="Logs"
								/>
							)}
							{!guardContent && (
								<WithTooltip
									delayDuration={0}
									disabled={showLabels}
									trigger={
										<TabsTrigger
											value="console"
											className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
										>
											<Icon
												icon={faTerminal}
												className="shrink-0"
											/>
											<span
												className={
													showLabels
														? "truncate"
														: "hidden"
												}
											>
												Console
											</span>
										</TabsTrigger>
									}
									content="Console"
								/>
							)}
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										value="metadata"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faTag}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Metadata
										</span>
										<QueriedActorStatusIndicator
											className="absolute top-0.5 right-0"
											actorId={actorId}
										/>
									</TabsTrigger>
								}
								content="Metadata"
							/>
						</TabsList>
					</div>
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
			<TabsContent
				value="deployment-logs"
				className="min-h-0 flex-1 mt-0 h-full"
			>
				<DeploymentLogs pool="default" filter={`actorId=${actorId}`} />
			</TabsContent>
			<TabsContent value="console" className="min-h-0 flex-1 mt-0 h-full">
				{guardContent || (
					<div className="flex flex-col h-full">
						<ActorWorkerContextProvider actorId={actorId}>
							<ActorConsoleFull actorId={actorId} />
						</ActorWorkerContextProvider>
					</div>
				)}
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
	const { ref: tabListRef, showLabels } = useShowTabLabels();

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
					<div
						ref={tabListRef}
						className="flex-1 min-w-0 overflow-hidden h-full"
					>
						<TabsList className="flex border-none h-full items-end min-w-0 overflow-hidden w-full">
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										disabled={disabled}
										value="workflow"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faDiagramProject}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Workflow
										</span>
									</TabsTrigger>
								}
								content="Workflow"
							/>
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										disabled={disabled}
										value="database"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faDatabase}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Database
										</span>
									</TabsTrigger>
								}
								content="Database"
							/>
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										disabled={disabled}
										value="state"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faCubesStacked}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											State
										</span>
									</TabsTrigger>
								}
								content="State"
							/>
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										disabled={disabled}
										value="queue"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faInbox}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Queue
										</span>
									</TabsTrigger>
								}
								content="Queue"
							/>
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										disabled={disabled}
										value="connections"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faPlug}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Connections
										</span>
									</TabsTrigger>
								}
								content="Connections"
							/>
							<WithTooltip
								delayDuration={0}
								disabled={showLabels}
								trigger={
									<TabsTrigger
										disabled={disabled}
										value="metadata"
										className="text-xs px-2 py-1 pb-2 min-w-0 shrink gap-1"
									>
										<Icon
											icon={faTag}
											className="shrink-0"
										/>
										<span
											className={
												showLabels
													? "truncate"
													: "hidden"
											}
										>
											Metadata
										</span>
									</TabsTrigger>
								}
								content="Metadata"
							/>
						</TabsList>
					</div>
				</div>
			</div>
			{children}
		</Tabs>
	);
}
