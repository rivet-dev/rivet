import {
	Flex,
	LogsView,
	Skeleton,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	cn,
} from "@rivet-gg/components";
import { motion } from "framer-motion";
import { memo, Suspense } from "react";
import { ActorConfigTab } from "./actor-config-tab";
import { ActorConnectionsTab } from "./actor-connections-tab";
import { ActorDetailsSettingsProvider } from "./actor-details-settings";
import { ActorLogsTab } from "./actor-logs-tab";
import { ActorStateTab } from "./actor-state-tab";
import { ActorStatus } from "./actor-status";
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

interface ActorsActorDetailsProps {
	className?: string;
	tab?: string;
	actor: ActorAtom;
	onTabChange?: (tab: string) => void;
}

export const ActorsActorDetails = memo(
	({ tab, onTabChange, actor, className }: ActorsActorDetailsProps) => {
		const actorFeatures = useAtomValue(currentActorFeaturesAtom);

		const supportsState = actorFeatures?.includes(ActorFeature.State);
		const supportsLogs = actorFeatures?.includes(ActorFeature.Logs);
		const supportsConnections = actorFeatures?.includes(
			ActorFeature.Connections,
		);
		const supportsConfig = actorFeatures?.includes(ActorFeature.Config);
		const supportsConsole = actorFeatures?.includes(ActorFeature.Console);

		return (
			<ActorDetailsSettingsProvider>
				<ActorWorkerContextProvider actor={actor}>
					<div className="flex flex-col h-full flex-1 pt-2">
						<Tabs
							value={tab || (supportsState ? "state" : "logs")}
							onValueChange={onTabChange}
							defaultValue={supportsState ? "state" : "logs"}
							className={cn(
								className,
								"flex-1 min-h-0 flex flex-col",
							)}
						>
							<motion.div className="flex justify-between items-center border-b">
								<ActorsSidebarToggleButton />
								<motion.div layout className="flex flex-1">
									<TabsList className="overflow-auto border-none">
										{supportsState ? (
											<TabsTrigger value="state">
												State
											</TabsTrigger>
										) : null}
										{supportsConnections ? (
											<TabsTrigger value="connections">
												Connections
											</TabsTrigger>
										) : null}
										{supportsLogs ? (
											<TabsTrigger value="logs">
												Logs
											</TabsTrigger>
										) : null}
										{supportsConfig ? (
											<TabsTrigger value="config">
												Config
											</TabsTrigger>
										) : null}
									</TabsList>
									<Flex
										gap="2"
										justify="between"
										items="center"
										className="h-[36px] pb-3 pt-2 pr-4"
									>
										<ActorStatus
											className="text-sm h-auto"
											actor={actor}
										/>
										<ActorStopButton actor={actor} />
									</Flex>
								</motion.div>
							</motion.div>
							{supportsLogs ? (
								<TabsContent
									value="logs"
									className="min-h-0 flex-1 mt-0 h-full"
								>
									<Suspense
										fallback={<ActorLogsTab.Skeleton />}
									>
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
						</Tabs>

						{supportsConsole ? <ActorConsole /> : null}
					</div>
				</ActorWorkerContextProvider>
			</ActorDetailsSettingsProvider>
		);
	},
);

// @ts-expect-error
ActorsActorDetails.Skeleton = () => {
	return (
		<Flex className="h-full flex-col">
			<div className="flex flex-col gap-4 px-4 flex-wrap">
				<Skeleton className="mt-3 mx-auto h-10 w-full" />
				<div className="flex gap-2 items-center">
					<Skeleton className="h-6 w-1/4" />
					<Skeleton className="h-6 w-1/4" />
					<Skeleton className="h-6 w-1/4" />
					<Skeleton className="h-6 w-1/5" />
				</div>
				<div className="flex gap-2 flex-col">
					<Skeleton className="h-6 w-1/6" />
					<Skeleton className="h-6 w-1/4" />
				</div>
			</div>
			<div className="mt-4 flex gap-1 px-4">
				<Skeleton className="h-6 w-1/5" />
				<Skeleton className="h-6 w-1/5" />
				<Skeleton className="h-6 w-1/5" />
			</div>
			<div className="px-4 pt-4">
				<LogsView.Skeleton />
			</div>
		</Flex>
	);
};
