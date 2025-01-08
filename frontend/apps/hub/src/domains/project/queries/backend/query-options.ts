import { mergeWatchStreams } from "@/lib/watch-utilities";
import { rivetEeClient } from "@/queries/global";
import { getMetaWatchIndex } from "@/queries/utils";
import { queryOptions } from "@tanstack/react-query";
import { z } from "zod";
import { BackendEvent } from "./types";

const partialEnvLogsResponse = z
	.object({
		events: z.array(z.object({}).passthrough()),
	})
	.passthrough();

export const projectBackendQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) =>
	queryOptions({
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"backend",
		],
		queryFn: ({ queryKey: [_, projectId, __, environmentId], signal }) =>
			rivetEeClient.ee.backend.get(
				projectId,
				environmentId,
				{},
				{ abortSignal: signal },
			),
		select: (data) => data.backend,
	});

export const projectBackendEnvVariablesQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) =>
	queryOptions({
		queryKey: [
			"project",
			projectId,
			"backend-env",
			environmentId,
			"variables",
		],
		queryFn: ({ queryKey: [_, projectId, __, environmentId], signal }) =>
			rivetEeClient.ee.backend.getVariables(projectId, environmentId, {
				abortSignal: signal,
			}),
		select: (data) => data.variables,
	});

export const projectBackendEnvEventsQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) =>
	queryOptions({
		queryKey: [
			"project",
			projectId,
			"backend-env",
			environmentId,
			"events",
		],
		queryFn: async ({
			queryKey: [_, projectId, __, environmentId],
			meta,
			signal,
		}) => {
			const response = await rivetEeClient.ee.backend.getEvents(
				projectId,
				environmentId,
				{ watchIndex: getMetaWatchIndex(meta) },
				{ abortSignal: signal },
			);
			return {
				...response,
				events: z.array(BackendEvent).parse(response.events),
			};
		},
		select: (data) => data.events,
		meta: {
			watch: mergeWatchStreams,
		},
	});

export const projectBackendEnvEventQueryOptions = ({
	projectId,
	environmentId,
	eventId,
}: {
	projectId: string;
	environmentId: string;
	eventId: string;
}) =>
	queryOptions({
		...projectBackendEnvEventsQueryOptions({ projectId, environmentId }),
		select: (data) =>
			data.events.find((event) => event.eventTimestamp === eventId),
	});

export const projectBackendProjectEnvDatabaseQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) =>
	queryOptions({
		queryKey: [
			"project",
			projectId,
			"backend-env",
			environmentId,
			"database-url",
		],
		queryFn: ({ queryKey: [_, projectId, __, environmentId] }) =>
			rivetEeClient.ee.backend.getDbUrl(projectId, environmentId),
	});

/**
 * Used only for storing query key
 */
export const projectBackendProjectEnvDatabasePreviewQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) =>
	queryOptions({
		gcTime: Number.POSITIVE_INFINITY,
		staleTime: Number.POSITIVE_INFINITY,
		enabled: false,
		queryKey: [
			"project",
			projectId,
			"backend-env",
			environmentId,
			"database",
			"preview",
		],
	});
