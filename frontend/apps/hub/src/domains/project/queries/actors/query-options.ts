import { mergeWatchStreams } from "@/lib/watch-utilities";
import { rivetClient } from "@/queries/global";
import { getMetaWatchIndex } from "@/queries/utils";
import { Rivet } from "@rivet-gg/api";
import {
	type InfiniteData,
	infiniteQueryOptions,
	queryOptions,
} from "@tanstack/react-query";
import { uniqueId } from "lodash";

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
		initialPageParam: "",
		queryFn: ({
			signal: abortSignal,
			pageParam,
			queryKey: [, project, , environment, , { includeDestroyed, tags }],
		}) =>
			rivetClient.actor.list(
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
				data: InfiniteData<Rivet.actor.ListActorsResponse>,
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
			rivetClient.actor.get(
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
			endpoint: createActorEndpoint(data.actor.network),
		}),
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
		stream,
	}: {
		projectNameId: string;
		environmentNameId: string;
		actorId: string;
		stream: Rivet.actor.LogStream;
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
			stream,
		],
		queryFn: async ({
			signal: abortSignal,
			meta,
			queryKey: [, project, , environment, , actorId, , stream],
		}) => {
			const response = await rivetClient.actor.logs.get(
				actorId,
				{
					project,
					environment,
					stream: stream as Rivet.actor.LogStream,
					watchIndex: getMetaWatchIndex(meta),
				},
				{ abortSignal },
			);
			return {
				...response,
				timestamps: response.timestamps.map((ts) => ts.toISOString()),
				lines: response.lines.map((line) => window.atob(line)),
				ids: response.timestamps.map(() => uniqueId(stream)),
			};
		},
		meta: {
			watch: mergeWatchStreams,
		},
	});
};

export const actorErrorsQueryOptions = ({
	projectNameId,
	environmentNameId,
	actorId,
}: {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
}) => {
	return queryOptions({
		...actorLogsQueryOptions({
			projectNameId,
			environmentNameId,
			actorId,
			stream: Rivet.actor.LogStream.StdErr,
		}),
		select: (data) => data.lines.length > 0,
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
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				project,
				__,
				environment,
				___,
				tagsJson,
			],
			signal: abortSignal,
		}) =>
			rivetClient.actor.builds.list(
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
			rivetClient.actor.builds.get(
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
			rivetClient.actor.regions.list(
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

const createActorEndpoint = (
	network: Rivet.actor.Network,
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
			const response = await rivetClient.actor.list(
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
