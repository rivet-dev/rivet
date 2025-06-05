import { mergeWatchStreams } from "@/lib/watch-utilities";
import { rivetClient } from "@/queries/global";
import { getMetaWatchIndex } from "@/queries/utils";
import type { Rivet } from "@rivet-gg/api-full";
import { safe, logfmt, type LogFmtValue, toRecord } from "@rivet-gg/components";
import { getActorStatus } from "@rivet-gg/components/actors";
import {
	type InfiniteData,
	infiniteQueryOptions,
	keepPreviousData,
	queryOptions,
} from "@tanstack/react-query";
import stripAnsi from 'strip-ansi';

export const projectActorsQueryOptions = ({
	projectNameId,
	environmentNameId,
	includeDestroyed,
	tags,
}: {
	projectNameId: string;
	environmentNameId: string;
	includeDestroyed?: boolean;
	tags?: Record<string, string>;
}) => {
	return infiniteQueryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"actors",
			{ includeDestroyed, tags },
		] as const,
		refetchInterval: 5000,
		staleTime: 0,
		gcTime: 0,
		initialPageParam: "",
		queryFn: ({
			signal: abortSignal,
			pageParam,
			queryKey: [, project, , environment, , { includeDestroyed, tags }],
		}) =>
			rivetClient.actors.list(
				{
					project,
					environment,
					includeDestroyed,
					cursor: pageParam ? pageParam : undefined,
					tagsJson: JSON.stringify(tags),
				},
				{ abortSignal },
			),
		select: (data) => data.pages.flatMap((page) => page.actors || []),
		getNextPageParam: (lastPage) => {
			return lastPage.pagination.cursor;
		},
		meta: {
			updateCache: (
				data: InfiniteData<Rivet.actors.ListActorsResponse>,
				client,
			) => {
				for (const page of data.pages) {
					for (const actor of page.actors) {
						client.setQueryData(
							[
								"project",
								projectNameId,
								"environment",
								environmentNameId,
								"actor",
								actor.id,
							],
							(oldData) => {
								if (!oldData) return oldData;
								return {
									actor,
								};
							},
						);
					}
				}
			},
		},
	});
};

export const actorsCountQueryOptions = ({
	projectNameId,
	environmentNameId,
	includeDestroyed,
	tags,
}: {
	projectNameId: string;
	environmentNameId: string;
	includeDestroyed?: boolean;
	tags?: Record<string, string>;
}) => {
	return infiniteQueryOptions({
		...projectActorsQueryOptions({
			projectNameId,
			environmentNameId,
			tags,
			includeDestroyed,
		}),
		select: (data) =>
			data.pages.flatMap((page) => page.actors || []).length,
		notifyOnChangeProps: ["data"],
	});
};

export const actorQueryOptions = ({
	projectNameId,
	environmentNameId,
	actorId,
}: {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"actor",
			actorId,
		],
		queryFn: ({
			signal: abortSignal,
			queryKey: [_, project, __, environment, ___, actorId],
		}) =>
			rivetClient.actors.get(
				actorId,
				{ project, environment },
				{
					abortSignal,
				},
			),
		select: (data) => ({
			...data.actor,
			createTs: data.actor.createdAt
				? new Date(data.actor.createdAt)
				: new Date(),
			startTs: data.actor.startedAt
				? new Date(data.actor.startedAt)
				: undefined,
			destroyTs: data.actor.destroyedAt
				? new Date(data.actor.destroyedAt)
				: undefined,
			runtime: {
				...data.actor.runtime,
				arguments: data.actor.runtime.arguments?.filter(
					(arg) => arg !== "",
				),
			},
			tags: data.actor.tags as Record<string, string>,
			endpoint: createActorEndpoint(data.actor.network),
		}),
	});
};

export const actorStatusQueryOptions = ({
	projectNameId,
	environmentNameId,
	actorId,
}: {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
}) => {
	return queryOptions({
		...actorQueryOptions({ projectNameId, environmentNameId, actorId }),
		select: (data) => getActorStatus(data.actor),
	});
};

export const actorDestroyedAtQueryOptions = ({
	projectNameId,
	environmentNameId,
	actorId,
}: {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
}) =>
	queryOptions({
		...actorQueryOptions({ projectNameId, environmentNameId, actorId }),
		select: (data) =>
			data.actor.destroyedAt
				? new Date(data.actor.destroyedAt)
				: undefined,
	});

export const actorLogsQueryOptions = (
	{
		projectNameId,
		environmentNameId,
		actorId,
	}: {
		projectNameId: string;
		environmentNameId: string;
		actorId: string;
	},
	opts: { refetchInterval?: number } = {},
) => {
	return queryOptions({
		...opts,
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"actor",
			actorId,
			"logs",
		] as const,
		queryFn: async ({
			signal: abortSignal,
			meta,
			queryKey: [, project, , environment, , actorId],
		}) => {
			const response = await rivetClient.actors.logs.get(
				{
					project,
					environment,
					actorIdsJson: JSON.stringify([actorId]),
					watchIndex: getMetaWatchIndex(meta),
					stream: "all",
				},
				{ abortSignal },
			);

			const logs = response.lines.map((line, idx) => {
				const timestamp = response.timestamps[idx];
				const stream = response.streams[idx];
				const raw = stripAnsi(window.atob(line));
				return {
					id: `${actorId}-${timestamp}-${idx}`,
					level: stream === 1 ? "error" : "info",
					timestamp,
					line: raw,
					message: "",
					properties: {},
				} as const
			});


			return {...response, logs };
		},
		meta: {
			watch: mergeWatchStreams,
		},
	});
};

export const actorBuildsQueryOptions = ({
	projectNameId,
	environmentNameId,
	tags = {},
}: {
	projectNameId: string;
	environmentNameId: string;
	tags?: Record<string, string>;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"actor-builds",
			tags,
		] as const,
		refetchInterval: 2000,
		queryFn: ({
			queryKey: [, project, , environment, , tagsJson],
			signal: abortSignal,
		}) =>
			rivetClient.builds.list(
				{ project, environment, tagsJson: JSON.stringify(tagsJson) },
				{
					abortSignal,
				},
			),
		select: (data) => data.builds,
	});
};

export const actorBuildTagsQueryOptions = ({
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
			"builds",
			{},
			"all",
		] as const,
		queryFn: async ({
			queryKey: [_, projectId, __, environmentId, ___, tags],
			signal: abortSignal,
		}) => {
			const response = await rivetClient.servers.builds.list(
				projectId,
				environmentId,
				{ tagsJson: JSON.stringify({}) },
				{
					abortSignal,
				},
			);

			return response.builds.flatMap((build) =>
				Object.entries(build.tags).map(([key, value]) => ({
					key,
					value,
				})),
			);
		},
		structuralSharing(oldData, newData) {
			const response =
				(newData as { key: string; value: string }[]) || [];

			const tags = new Map<string, Set<string>>();

			if (oldData && Array.isArray(oldData)) {
				for (const build of oldData) {
					for (const [key, value] of Object.entries(build.tags)) {
						if (!tags.has(key)) {
							tags.set(key, new Set<string>());
						}
						if (typeof value === "string") {
							tags.get(key)?.add(value);
						}
					}
				}
			}

			for (const { key, value } of response) {
				if (!tags.has(key)) {
					tags.set(key, new Set<string>());
				}
				tags.get(key)?.add(value);
			}

			const allTags = [...tags.entries()].flatMap(([key, values]) =>
				[...values.values()].map((value) => ({
					key,
					value,
				})),
			);

			return allTags;
		},
	});
};

export const actorBuildQueryOptions = ({
	projectNameId,
	environmentNameId,
	buildId,
}: {
	projectNameId: string;
	environmentNameId: string;
	buildId: string;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"actor-build",
			buildId,
		],
		queryFn: ({
			signal: abortSignal,
			queryKey: [_, project, __, environment, ___, build],
		}) =>
			rivetClient.builds.get(
				build,
				{ project, environment },
				{
					abortSignal,
				},
			),
		select: (data) => data.build,
	});
};

export const actorRegionsQueryOptions = ({
	projectNameId,
	environmentNameId,
}: { projectNameId: string; environmentNameId: string }) => {
	return queryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"regions",
		],
		queryFn: ({
			signal: abortSignal,
			queryKey: [_, project, __, environment],
		}) =>
			rivetClient.regions.list(
				{ project, environment },
				{
					abortSignal,
				},
			),
		select: (data) => data.regions,
	});
};

export const actorRegionQueryOptions = ({
	projectNameId,
	environmentNameId,
	regionId,
}: {
	projectNameId: string;
	environmentNameId: string;
	regionId: string;
}) => {
	return queryOptions({
		...actorRegionsQueryOptions({ projectNameId, environmentNameId }),
		select: (data) =>
			actorRegionsQueryOptions({ projectNameId, environmentNameId })
				.select?.(data)
				.find((region) => region.id === regionId),
	});
};

export const createActorEndpoint = (
	network: Rivet.actors.Network,
): string | undefined => {
	try {
		const http = Object.values(network.ports).find(
			(port) => port.protocol === "http" || port.protocol === "https",
		);
		if (!http) {
			return undefined;
		}
		// undocumented
		// @ts-ignore
		if (http.url) {
			// undocumented
			// @ts-ignore
			return http.url;
		}
		const url = new URL(`${http.protocol}://${http.hostname}:${http.port}`);
		url.pathname = http.path || "/";
		return url.href;
	} catch {
		return undefined;
	}
};

export const actorManagerUrlQueryOptions = ({
	projectNameId,
	environmentNameId,
}: {
	projectNameId: string;
	environmentNameId: string;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"actor-manager",
		],
		queryFn: async ({
			signal: abortSignal,
			queryKey: [_, project, __, environment],
		}) => {
			const response = await rivetClient.actors.list(
				{
					project,
					environment,
					tagsJson: JSON.stringify({ name: "manager" }),
				},
				{ abortSignal },
			);

			if (response.actors.length === 0) {
				throw new Error("No actor manager found");
			}
			const href = createActorEndpoint(response.actors[0].network);

			if (!href) {
				throw new Error("No actor manager found");
			}

			return href;
		},
	});
};

export const actorBuildsCountQueryOptions = ({
	projectNameId,
	environmentNameId,
}: {
	projectNameId: string;
	environmentNameId: string;
}) => {
	return queryOptions({
		...actorBuildsQueryOptions({ projectNameId, environmentNameId }),
		select: (data) => data.builds.length,
		notifyOnChangeProps: ["data"],
	});
};

export interface FunctionInvoke {
	id: string;
	isFormatted: boolean;
	actorId: string;
	actorName: string;
	actorTags: Record<string, unknown>;
	regionId: string;
	timestamp: Date;
	level: string;
	line: string;
	message: string;
	properties: Record<string, LogFmtValue>;
}

export const logsAggregatedQueryOptions = ({
	projectNameId,
	environmentNameId,
	search,
}: {
	projectNameId: string;
	environmentNameId: string;
	search?: { text?: string; caseSensitive?: boolean; enableRegex?: boolean };
}) => {
	return queryOptions({
		refetchInterval: 5000,
		placeholderData: keepPreviousData,
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"logs",
			search,
		] as const,
		queryFn: async ({
			signal: abortSignal,
			client,
			queryKey: [_, project, __, environment, ___, search],
		}) => {
			const actors = await client.fetchInfiniteQuery({
				...projectActorsQueryOptions({
					projectNameId: project,
					environmentNameId: environment,
					includeDestroyed: true,
					tags: {},
				}),
				pages: 10,
			});

			const allActors = actors.pages.flatMap((page) => page.actors || []);

			const actorsMap = new Map<string, Rivet.actors.Actor>();
			for (const actor of allActors) {
				actorsMap.set(actor.id, actor);
			}

			const logs = await rivetClient.actors.logs.get(
				{
					stream: "all",
					project,
					environment,
					searchText: search?.text,
					searchCaseSensitive: search?.caseSensitive,
					searchEnableRegex: search?.enableRegex,
					actorIdsJson: JSON.stringify(allActors.map((a) => a.id)),
				},
				{ abortSignal },
			);

			const parsed = logs.lines.map((line, idx) => {
				const actorIdx = logs.actorIndices[idx];
				const actorId = logs.actorIds[actorIdx];
				const timestamp = logs.timestamps[idx];
				const stream = logs.streams[idx];
				const raw = stripAnsi(window.atob(line));
				const fmt = safe(logfmt.parse)(raw)[0];
				const json = safe(JSON.parse)(raw)[0];
				const formatted = json || fmt;
				const {
					level = stream === 1 ? "error" : "info",
					msg,
					...properties
				} = formatted || {};
				const isFormatDefined =
					(fmt?.level || json) && Object.keys(formatted).length > 0;
				const actor = actorsMap.get(actorId);
				return {
					id: `${actorId}-${timestamp}-${idx}`,
					level: level,
					isFormatted: isFormatDefined,
					actorId: actorId,
					actorName:
						(toRecord(actor?.tags).name as string) ||
						actorId.split("-")[0],
					actorTags: toRecord(actor?.tags),
					regionId: actor?.region || "local",
					timestamp,
					line: raw,
					message: isFormatDefined ? (msg as string) : "",
					properties: isFormatDefined ? properties : {},
				} satisfies FunctionInvoke;
			});

			return parsed.toReversed();
		},
	});
};

export interface Route {
	id: string;
	hostname: string;
	pathPrefix: string;
	selector: Record<string, string>;
	createdAt: Date;
}

export const routesQueryOptions = ({
	projectNameId,
	environmentNameId,
}: {
	projectNameId: string;
	environmentNameId: string;
}) => {
	return queryOptions({
		queryKey: [
			"project",
			projectNameId,
			"environment",
			environmentNameId,
			"routes",
		],
		queryFn: async ({
			signal: abortSignal,
			queryKey: [_, project, __, environment],
		}) => {
			return rivetClient.routes.list(
				{
					project,
					environment,
				},
				{ abortSignal },
			);
		},
		select: (data) => data.routes,
	});
};

export const routeQueryOptions = ({
	projectNameId,
	environmentNameId,
	routeId,
}: {
	projectNameId: string;
	environmentNameId: string;
	routeId: string;
}) => {
	return queryOptions({
		...routesQueryOptions({
			projectNameId,
			environmentNameId,
		}),
		select: (data) => {
			return data.routes.find((route) => route.id === routeId);
		},
	});
};
