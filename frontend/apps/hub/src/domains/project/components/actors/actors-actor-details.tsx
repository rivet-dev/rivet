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
import { useSuspenseQuery } from "@tanstack/react-query";
import { motion } from "framer-motion";
import { Suspense } from "react";
import { actorQueryOptions, buildQueryOptions } from "../../queries";
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
import { ACTOR_FRAMEWORK_TAG_VALUE } from "./actor-tags";
import { useEnvironment } from "../../data/environment-context";
import { useProject } from "../../data/project-context";
import { toRecord } from "@/lib/utils";

interface ActorsActorDetailsProps {
	actorId: string;
	className?: string;
	tab?: string;
	onTabChange?: (tab: string) => void;
}

export function ActorsActorDetails({
	tab,
	onTabChange,
	className,
	actorId,
	...props
}: ActorsActorDetailsProps) {
	const { nameId: projectNameId, gameId: projectId } = useProject();
	const { nameId: environmentNameId, namespaceId: environmentId } =
		useEnvironment();

	const { data } = useSuspenseQuery(
		actorQueryOptions({
			projectNameId,
			environmentNameId,
			actorId,
		}),
	);
	const { data: build } = useSuspenseQuery(
		buildQueryOptions({
			projectId,
			environmentId,
			buildId: data.runtime.build,
		}),
	);

	const isActorCore =
		toRecord(build?.tags).framework === ACTOR_FRAMEWORK_TAG_VALUE;

	return (
		<ActorDetailsSettingsProvider>
			<ActorWorkerContextProvider
				enabled={!data.destroyedAt && isActorCore}
				endpoint={data.endpoint}
				projectNameId={projectNameId}
				environmentNameId={environmentNameId}
				actorId={actorId}
			>
				<div className="flex flex-col h-full flex-1 pt-2">
					<Tabs
						value={tab || "logs"}
						onValueChange={onTabChange}
						defaultValue="logs"
						className={cn(
							className,
							"flex-1 min-h-0 flex flex-col",
						)}
					>
						<motion.div className="flex justify-between items-center border-b">
							<ActorsSidebarToggleButton />
							<motion.div layout className="flex flex-1">
								<TabsList className="overflow-auto border-none">
									<TabsTrigger value="logs">Logs</TabsTrigger>
									<TabsTrigger value="config">
										Config
									</TabsTrigger>
									{isActorCore ? (
										<>
											<TabsTrigger value="state">
												State
											</TabsTrigger>
											<TabsTrigger value="connections">
												Connections
											</TabsTrigger>
										</>
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
										{...data}
									/>
									<ActorStopButton
										projectNameId={projectNameId}
										environmentNameId={environmentNameId}
										actorId={actorId}
									/>
								</Flex>
							</motion.div>
						</motion.div>
						<TabsContent
							value="logs"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							<Suspense fallback={<ActorLogsTab.Skeleton />}>
								<ActorLogsTab
									projectNameId={projectNameId}
									environmentNameId={environmentNameId}
									actorId={actorId}
									{...data}
								/>
							</Suspense>
						</TabsContent>
						<TabsContent
							value="config"
							className="min-h-0 flex-1 mt-0 h-full"
						>
							<ActorConfigTab
								projectNameId={projectNameId}
								environmentNameId={environmentNameId}
								{...data}
							/>
						</TabsContent>
						<TabsContent
							value="connections"
							className="min-h-0 flex-1 mt-0"
						>
							<ActorConnectionsTab
								disabled={!!data.destroyTs || !data.startedAt}
							/>
						</TabsContent>
						<TabsContent
							value="state"
							className="min-h-0 flex-1 mt-0"
						>
							<ActorStateTab
								disabled={!!data.destroyTs || !data.startedAt}
							/>
						</TabsContent>
					</Tabs>

					{!data.destroyTs && isActorCore ? <ActorConsole /> : null}
				</div>
			</ActorWorkerContextProvider>
		</ActorDetailsSettingsProvider>
	);
}

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
