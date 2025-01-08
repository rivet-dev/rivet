import { rivetClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api";
import { queryOptions } from "@tanstack/react-query";
import { getLiveLobbyStatus, getLobbyStatus } from "../../data/lobby-status";
import { projectQueryOptions } from "../query-options";

export const projectEnvironmentsQueryOptions = (projectId: string) => {
	return queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			// biome-ignore lint/style/noNonNullAssertion: when we get here, we know the project exists
			projectQueryOptions(projectId).select?.(data).namespaces!,
	});
};

export const projectEnvironmentQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		queryKey: ["project", projectId, "environment", environmentId],
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				__,
				environmentId,
			],
			signal,
		}) =>
			rivetClient.cloud.games.namespaces.getGameNamespaceById(
				projectId,
				environmentId,
				{ abortSignal: signal },
			),
	});
};

export const projectEnvironmentDisplayNameQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) =>
	queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			projectQueryOptions(projectId)
				.select?.(data)
				.namespaces.find(
					(namespace) => namespace.namespaceId === environmentId,
				)?.displayName,
	});

export const projectEnvironmentVersionQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			projectQueryOptions(projectId)
				.select?.(data)
				.namespaces.find(
					(namespace) => namespace.namespaceId === environmentId,
				)?.version,
	});
};

export const projectEnvironmentLobbyQueryOptions = (
	{
		projectId,
		environmentId,
		lobbyId,
	}: {
		projectId: string;
		environmentId: string;
		lobbyId: string;
	},
	opts?: { refetchInterval?: number; throwOnError?: boolean },
) => {
	return queryOptions({
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"lobby",
			lobbyId,
		],
		refetchInterval: opts?.refetchInterval,
		throwOnError: opts?.throwOnError,
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				__,
				environmentId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				___,
				lobbyId,
			],
			signal,
		}) =>
			rivetClient.cloud.games.namespaces.logs.getNamespaceLobby(
				projectId,
				environmentId,
				lobbyId,
				{ abortSignal: signal },
			),
		select: (data) => ({
			...data,
			lobby: {
				...data.lobby,
				readableStatus: getLobbyStatus(
					data.lobby.status,
					data.lobby.startTs,
				),
				stopTs: data.lobby.status.stopped?.stopTs,
			},
		}),
	});
};

export const projectEnvironmentTokenPublicQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		staleTime: 0,
		gcTime: 0,
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"token",
			"public",
		],
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				__,
				environmentId,
			],
			signal,
		}) =>
			rivetClient.cloud.games.namespaces.createGameNamespaceTokenPublic(
				projectId,
				environmentId,
				{ abortSignal: signal },
			),
		select: (data) => data.token,
	});
};

export const projectEnvironmentLogsLobbiesQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"lobbies",
		],
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				__,
				environmentId,
			],
			signal,
		}) =>
			rivetClient.cloud.games.namespaces.logs.listNamespaceLobbies(
				projectId,
				environmentId,
				{},
				{ abortSignal: signal },
			),
		select: (data) =>
			data.lobbies.map((lobby) => ({
				...lobby,
				readableStatus: getLobbyStatus(lobby.status, lobby.startTs),
			})),
	});
};

export const projectEnvironmentLogsLobbyLogsQueryOptions = (
	{
		projectId,
		lobbyId,
		stream,
	}: {
		projectId: string;
		lobbyId: string;
	} & Rivet.cloud.games.GetLobbyLogsRequest,
	opts?: { refetchInterval?: number },
) => {
	return queryOptions({
		// watch does not work on this query
		refetchInterval: opts?.refetchInterval,
		queryKey: ["project", projectId, "lobby", lobbyId, "logs", stream],
		queryFn: async ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				__,
				lobbyId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				___,
				stream,
			],
			signal,
		}) => {
			const response =
				await rivetClient.cloud.games.matchmaker.getLobbyLogs(
					projectId,
					lobbyId,
					{
						stream: stream as Rivet.cloud.games.LogStream,
					},
					{ abortSignal: signal },
				);
			return {
				...response,
				lines: response.lines.map((line) => window.atob(line)),
			};
		},
	});
};

export const projectEnvironmentLobbiesLiveQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"lobbies",
			"live",
		],
		refetchInterval: 1000,
		queryFn: async ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				__,
				environmentId,
			],
			signal,
		}) =>
			rivetClient.cloud.games.namespaces.analytics.getAnalyticsMatchmakerLive(
				projectId,
				environmentId,
				{ abortSignal: signal },
			),
		select: (data) => ({
			...data,
			lobbies: data.lobbies
				.map((lobby) => ({
					...lobby,
					readableStatus: getLiveLobbyStatus(lobby),
				}))
				.sort((a, b) => {
					// sort by created time
					return +b.createTs - +a.createTs;
				}),
		}),
	});
};
