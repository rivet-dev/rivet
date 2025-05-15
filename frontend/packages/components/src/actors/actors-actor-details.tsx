import {
	Flex,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	cn,
} from "@rivet-gg/components";
import { memo, type ReactNode, Suspense } from "react";
import { ActorConfigTab } from "./actor-config-tab";
import { ActorConnectionsTab } from "./actor-connections-tab";
import { ActorDetailsSettingsProvider } from "./actor-details-settings";
import { ActorLogsTab } from "./actor-logs-tab";
import { ActorStateTab } from "./actor-state-tab";
import { AtomizedActorStatus } from "./actor-status";
import { ActorStopButton } from "./actor-stop-button";
import { ActorsSidebarToggleButton } from "./actors-sidebar-toggle-button";
import { ActorConsole } from "./console/actor-console";
import { ActorWorkerContextProvider } from "./worker/actor-worker-context";
import {
	ActorFeature,
	currentActorFeaturesAtom,
	type ActorAtom,
} from "./actor-context";
import { useAtomValue } from "jotai";
import { useActorsView } from "./actors-view-context-provider";
import { faQuestionSquare, Icon } from "@rivet-gg/icons";

interface ActorsActorDetailsProps {
	tab?: string;
	actor: ActorAtom;
	onTabChange?: (tab: string) => void;
}

export const ActorsActorDetails = memo(
	({ tab, onTabChange, actor }: ActorsActorDetailsProps) => {
		const actorFeatures = useAtomValue(currentActorFeaturesAtom);
		const supportsConsole = actorFeatures?.includes(ActorFeature.Console);

		return (
			<ActorDetailsSettingsProvider>
				<ActorWorkerContextProvider
					actor={actor}
					notifyOnReconnect={actorFeatures?.includes(
						ActorFeature.InspectReconnectNotification,
					)}
				>
					<div className="flex flex-col h-full flex-1 pt-2">
						<ActorTabs
							features={actorFeatures}
							actor={actor}
							tab={tab}
							onTabChange={onTabChange}
						/>

						{supportsConsole ? <ActorConsole /> : null}
					</div>
				</ActorWorkerContextProvider>
			</ActorDetailsSettingsProvider>
		);
	},
);

export const ActorsActorEmptyDetails = ({
	features,
}: {
	features: ActorFeature[];
}) => {
	const { copy } = useActorsView();
	return (
		<div className="flex flex-col h-full flex-1 pt-2">
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
	actor,
	className,
	disabled,
	children,
}: {
	disabled?: boolean;
	tab?: string;
	features: ActorFeature[];
	onTabChange?: (tab: string) => void;
	actor?: ActorAtom;
	className?: string;
	children?: ReactNode;
}) {
	const supportsState = features?.includes(ActorFeature.State);
	const supportsLogs = features?.includes(ActorFeature.Logs);
	const supportsConnections = features?.includes(ActorFeature.Connections);
	const supportsConfig = features?.includes(ActorFeature.Config);

	const defaultTab = supportsState ? "state" : "logs";
	const value = disabled ? undefined : tab || defaultTab;

	return (
		<Tabs
			value={value}
			onValueChange={onTabChange}
			defaultValue={value}
			className={cn(className, "flex-1 min-h-0 flex flex-col")}
		>
			<div className="flex justify-between items-center border-b">
				<ActorsSidebarToggleButton />
				<div className="flex flex-1">
					<TabsList className="overflow-auto border-none">
						{supportsState ? (
							<TabsTrigger disabled={disabled} value="state">
								State
							</TabsTrigger>
						) : null}
						{supportsConnections ? (
							<TabsTrigger
								disabled={disabled}
								value="connections"
							>
								Connections
							</TabsTrigger>
						) : null}
						{supportsLogs ? (
							<TabsTrigger disabled={disabled} value="logs">
								Logs
							</TabsTrigger>
						) : null}
						{supportsConfig ? (
							<TabsTrigger disabled={disabled} value="config">
								Config
							</TabsTrigger>
						) : null}
					</TabsList>
					{actor ? (
						<Flex
							gap="2"
							justify="between"
							items="center"
							className="h-[36px] pb-3 pt-2 pr-4"
						>
							<AtomizedActorStatus
								className="text-sm h-auto"
								actor={actor}
							/>
							<ActorStopButton actor={actor} />
						</Flex>
					) : null}
				</div>
			</div>
			{actor ? (
				<>
					{supportsLogs ? (
						<TabsContent
							value="logs"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							<Suspense fallback={<ActorLogsTab.Skeleton />}>
								<ActorLogsTab actor={actor} />
							</Suspense>
						</TabsContent>
					) : null}
					{supportsConfig ? (
						<TabsContent
							value="config"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							<ActorConfigTab actor={actor} />
						</TabsContent>
					) : null}
					{supportsConnections ? (
						<TabsContent
							value="connections"
							className="min-h-0 flex-1 mt-0"
						>
							<ActorConnectionsTab actor={actor} />
						</TabsContent>
					) : null}
					{supportsState ? (
						<TabsContent
							value="state"
							className="min-h-0 flex-1 mt-0"
						>
							<ActorStateTab actor={actor} />
						</TabsContent>
					) : null}
				</>
			) : null}
			{children}
		</Tabs>
	);
}
