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
import { ActorEventsTab } from "./actor-events-tab";
import { ActorLogsTab } from "./actor-logs-tab";
import { ActorMetricsTab } from "./actor-metrics-tab";
import { ActorStateTab } from "./actor-state-tab";
import { QueriedActorStatus } from "./actor-status";
import { ActorStopButton } from "./actor-stop-button";
import { ActorsSidebarToggleButton } from "./actors-sidebar-toggle-button";
import { useActorsView } from "./actors-view-context-provider";
import { ActorConsole } from "./console/actor-console";
import {
	GuardConnectableInspector,
	useInspectorGuard,
} from "./guard-connectable-inspector";
import { useManager } from "./manager-context";
import { ActorFeature, type ActorId } from "./queries";
import { ActorWorkerContextProvider } from "./worker/actor-worker-context";

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
		const { data: features = [] } = useQuery(
			useManager().actorFeaturesQueryOptions(actorId),
		);

		const supportsConsole = features.includes(ActorFeature.Console);

		return (
			<GuardConnectableInspector actorId={actorId}>
				<ActorDetailsSettingsProvider>
					<div className="flex flex-col h-full flex-1">
						<ActorTabs
							features={features}
							actorId={actorId}
							tab={tab}
							onTabChange={onTabChange}
						/>

						{supportsConsole ? <Console actorId={actorId} /> : null}
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

export const ActorsActorEmptyDetails = ({
	features,
}: {
	features: ActorFeature[];
}) => {
	const { copy } = useActorsView();
	return (
		<div className="flex flex-col h-full w-full min-w-0 min-h-0 flex-1">
			<ActorTabs disabled features={features}>
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col gap-2">
					<Icon icon={faQuestionSquare} className="text-4xl" />
					<p className="max-w-[400px]">{copy.selectActor}</p>
				</div>
			</ActorTabs>
		</div>
	);
};

export function ActorTabs({
	tab,
	features,
	onTabChange,
	actorId,
	className,
	disabled,
	children,
}: {
	disabled?: boolean;
	tab?: string;
	features: ActorFeature[];
	onTabChange?: (tab: string) => void;
	actorId?: ActorId;
	className?: string;
	children?: ReactNode;
}) {
	const supportsState = features?.includes(ActorFeature.State);
	const supportsLogs = features?.includes(ActorFeature.Logs);
	const supportsConnections = features?.includes(ActorFeature.Connections);
	const supportsMetadata = features?.includes(ActorFeature.Config);
	const supportsMetrics = features?.includes(ActorFeature.Metrics);
	const supportsEvents = features?.includes(ActorFeature.EventsMonitoring);
	const supportsDatabase = features?.includes(ActorFeature.Database);

	const defaultTab = supportsState ? "state" : "logs";
	const value = disabled ? undefined : tab || defaultTab;

	const guardContent = useInspectorGuard();

	return (
		<Tabs
			value={value}
			onValueChange={onTabChange}
			defaultValue={value}
			className={cn(className, "flex-1 min-h-0 min-w-0 flex flex-col ")}
		>
			<div className="flex justify-between items-center border-b h-[45px]">
				<ActorsSidebarToggleButton />
				<div className="flex flex-1 items-center h-full w-full ">
					<TabsList className="overflow-auto border-none h-full items-end">
						{supportsState ? (
							<TabsTrigger
								disabled={disabled}
								value="state"
								className="text-xs px-3 py-1 pb-2"
							>
								State
							</TabsTrigger>
						) : null}
						{supportsConnections ? (
							<TabsTrigger
								disabled={disabled}
								value="connections"
								className="text-xs px-3 py-1 pb-2"
							>
								Connections
							</TabsTrigger>
						) : null}
						{supportsEvents ? (
							<TabsTrigger
								disabled={disabled}
								value="events"
								className="text-xs px-3 py-1 pb-2"
							>
								Events
							</TabsTrigger>
						) : null}
						{supportsDatabase ? (
							<TabsTrigger
								disabled={disabled}
								value="database"
								className="text-xs px-3 py-1 pb-2"
							>
								Database
							</TabsTrigger>
						) : null}
						{supportsLogs ? (
							<TabsTrigger
								disabled={disabled}
								value="logs"
								className="text-xs px-3 py-1 pb-2"
							>
								Logs
							</TabsTrigger>
						) : null}
						{supportsMetadata ? (
							<TabsTrigger
								disabled={disabled}
								value="metadata"
								className="text-xs px-3 py-1 pb-2"
							>
								Metadata
							</TabsTrigger>
						) : null}
						{supportsMetrics ? (
							<TabsTrigger
								disabled={disabled}
								value="metrics"
								className="text-xs px-3 py-1 pb-2"
							>
								Metrics
							</TabsTrigger>
						) : null}
					</TabsList>
					{actorId ? (
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
					) : null}
				</div>
			</div>
			{actorId ? (
				<>
					{supportsLogs ? (
						<TabsContent
							value="logs"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							<Suspense fallback={<ActorLogsTab.Skeleton />}>
								{guardContent || (
									<ActorLogsTab actorId={actorId} />
								)}
							</Suspense>
						</TabsContent>
					) : null}
					{supportsMetadata ? (
						<TabsContent
							value="metadata"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							<ActorConfigTab actorId={actorId} />
						</TabsContent>
					) : null}
					{supportsConnections ? (
						<TabsContent
							value="connections"
							className="min-h-0 flex-1 mt-0"
						>
							{guardContent || (
								<ActorConnectionsTab actorId={actorId} />
							)}
						</TabsContent>
					) : null}
					{supportsEvents ? (
						<TabsContent
							value="events"
							className="min-h-0 flex-1 mt-0"
						>
							{guardContent || (
								<ActorEventsTab actorId={actorId} />
							)}
						</TabsContent>
					) : null}
					{supportsDatabase ? (
						<TabsContent
							value="database"
							className="min-h-0 min-w-0 flex-1 mt-0 h-full"
						>
							{guardContent || (
								<ActorDatabaseTab actorId={actorId} />
							)}
						</TabsContent>
					) : null}
					{supportsState ? (
						<TabsContent
							value="state"
							className="min-h-0 flex-1 mt-0"
						>
							{guardContent || (
								<ActorStateTab actorId={actorId} />
							)}
						</TabsContent>
					) : null}
					{supportsMetrics ? (
						<TabsContent
							value="metrics"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							{guardContent || (
								<ActorMetricsTab actorId={actorId} />
							)}
						</TabsContent>
					) : null}
				</>
			) : null}
			{children}
		</Tabs>
	);
}
