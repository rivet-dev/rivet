import {
	Flex,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	Text,
} from "@rivet-gg/components";
import { ErrorBoundary } from "@sentry/react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Suspense } from "react";
import { projectEnvironmentLobbyQueryOptions } from "../../queries";
import { LobbyLifecycle } from "./lobby-lifecycle";
import { LobbyLogs } from "./lobby-logs";
import { LobbyMetrics } from "./lobby-metrics";
import { LobbyStats } from "./lobby-stats";
import { LobbySummary } from "./lobby-summary";

interface ProjectMatchmakerLobbyDetailsPanelProps {
	lobbyId?: string;
	projectId: string;
	environmentId: string;
	isLive?: boolean;
}

export function ProjectMatchmakerLobbyDetailsPanel({
	lobbyId,
	projectId,
	environmentId,
	isLive,
}: ProjectMatchmakerLobbyDetailsPanelProps) {
	if (!lobbyId) {
		return (
			<Flex items="center" justify="center" className="h-full">
				<Text textAlign="center" my="8">
					Please select a lobby from the list on the left.
				</Text>
			</Flex>
		);
	}

	const {
		data: { lobby, metrics },
		dataUpdatedAt,
	} = useSuspenseQuery(
		projectEnvironmentLobbyQueryOptions(
			{ projectId, environmentId, lobbyId },
			{ refetchInterval: isLive ? 1000 : undefined, throwOnError: true },
		),
	);

	return (
		<ErrorBoundary
			fallback={
				<Flex items="center" justify="center" className="h-full">
					<Text textAlign="center">
						An error occurred while fetching lobby data.
					</Text>
				</Flex>
			}
		>
			<Suspense fallback={<LobbySummary.Skeleton />}>
				<LobbySummary
					{...lobby}
					projectId={projectId}
					isLive={isLive}
					rightSide={
						<>
							<LobbyLifecycle
								createTs={lobby.createTs}
								readyTs={lobby.readyTs || lobby.startTs}
								stopTs={lobby.status.stopped?.stopTs}
							/>

							{isLive && metrics ? (
								<LobbyMetrics lobbyId={lobbyId} {...metrics} />
							) : null}
						</>
					}
				/>
			</Suspense>

			<Tabs
				defaultValue="logs"
				className="flex-1 min-h-0 flex flex-col mt-4 @container"
			>
				<TabsList className="overflow-auto">
					<TabsTrigger value="logs">Output</TabsTrigger>
					<TabsTrigger value="errors">Error</TabsTrigger>
					{isLive && metrics ? (
						<TabsTrigger value="stats">Monitor</TabsTrigger>
					) : null}
				</TabsList>
				<TabsContent value="logs" className="min-h-0 flex-1 mt-0 p-4">
					<Suspense fallback={<LobbyLogs.Skeleton />}>
						<LobbyLogs
							logType="std_out"
							lobbyId={lobbyId}
							projectId={projectId}
							isLive={isLive}
						/>
					</Suspense>
				</TabsContent>
				<TabsContent value="errors" className="min-h-0 flex-1 mt-0 p-4">
					<Suspense fallback={<LobbyLogs.Skeleton />}>
						<LobbyLogs
							logType="std_err"
							lobbyId={lobbyId}
							projectId={projectId}
							isLive={isLive}
						/>
					</Suspense>
				</TabsContent>
				{isLive && metrics ? (
					<TabsContent value="stats" className="min-h-0 flex-1 mt-0">
						<Suspense fallback={<LobbyLogs.Skeleton />}>
							<LobbyStats
								lobbyId={lobbyId}
								metricsAt={dataUpdatedAt}
								{...metrics}
							/>
						</Suspense>
					</TabsContent>
				) : null}
			</Tabs>
		</ErrorBoundary>
	);
}

ProjectMatchmakerLobbyDetailsPanel.Skeleton = () => {
	return (
		<Flex className="h-full flex-col">
			<LobbySummary.Skeleton />
			<LobbyLogs.Skeleton />
		</Flex>
	);
};
