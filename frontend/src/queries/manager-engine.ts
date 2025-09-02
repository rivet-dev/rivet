import { ActorFeature } from "@rivetkit/core/inspector";
import { type Rivet, RivetClient } from "@rivetkit/engine-api-full";
import {
	infiniteQueryOptions,
	queryOptions,
	skipToken,
} from "@tanstack/react-query";
import z from "zod";
import { getConfig } from "@/components";
import type {
	Actor,
	ActorId,
	CrashPolicy,
	ManagerContext,
} from "@/components/actors";
import {
	ACTORS_PER_PAGE,
	ActorQueryOptionsSchema,
	createDefaultManagerContext,
} from "@/components/actors/manager-context";

const client = new RivetClient({
	baseUrl: () => getConfig().apiUrl,
	environment: "",
});

export { client as managerClient };

export const createEngineManagerContext = ({
	namespace,
}: {
	namespace: string;
}) => {
	const def = createDefaultManagerContext();
	return {
		...def,
		features: {
			canCreateActors: true,
			canDeleteActors: true,
		},
		managerStatusQueryOptions() {
			return queryOptions({
				...def.managerStatusQueryOptions(),
				enabled: true,
				queryFn: async () => {
					return true;
				},
			});
		},
		regionsQueryOptions() {
			return infiniteQueryOptions({
				...def.regionsQueryOptions(),
				enabled: true,
				queryFn: async () => {
					const data = await client.datacenters.list();
					return {
						regions: data.datacenters.map((dc) => ({
							id: dc.name,
							name: dc.name,
						})),
						pagination: data.pagination,
					};
				},
			});
		},
		regionQueryOptions(regionId: string) {
			return queryOptions({
				...def.regionQueryOptions(regionId),
				queryKey: ["region", regionId],
				queryFn: async ({ client }) => {
					const regions = await client.ensureInfiniteQueryData(
						this.regionsQueryOptions(),
					);

					for (const page of regions.pages) {
						for (const region of page.regions) {
							if (region.id === regionId) {
								return region;
							}
						}
					}

					throw new Error(`Region not found: ${regionId}`);
				},
			});
		},
		actorQueryOptions(actorId) {
			return queryOptions({
				...def.actorQueryOptions(actorId),
				queryKey: [namespace, "actor", actorId],
				enabled: true,
				queryFn: async ({ signal: abortSignal }) => {
					const data = await client.actorsGet(
						actorId,
						{ namespace },
						{ abortSignal },
					);

					return transformActor(data.actor);
				},
			});
		},
		actorsQueryOptions(opts) {
			return infiniteQueryOptions({
				...def.actorsQueryOptions(opts),
				queryKey: [namespace, "actors", opts],
				enabled: true,
				initialPageParam: undefined,
				queryFn: async ({
					signal: abortSignal,
					pageParam,
					queryKey: [, , _opts],
				}) => {
					const { success, data: opts } =
						ActorQueryOptionsSchema.safeParse(_opts || {});

					if (
						(opts?.n?.length === 0 || !opts?.n) &&
						(opts?.filters?.id?.value?.length === 0 ||
							!opts?.filters?.id?.value ||
							opts?.filters.key?.value?.length === 0 ||
							!opts?.filters.key?.value)
					) {
						// If there are no names specified, we can return an empty result
						return {
							actors: [],
							pagination: {
								cursor: undefined,
							},
						};
					}

					const data = await client.actorsList(
						{
							namespace,
							cursor: pageParam ?? undefined,
							actorIds: opts?.filters?.id?.value?.join(","),
							key: opts?.filters?.key?.value?.join(","),
							includeDestroyed:
								success &&
								(opts?.filters?.showDestroyed?.value.includes(
									"true",
								) ||
									opts?.filters?.showDestroyed?.value.includes(
										"1",
									)),
							limit: ACTORS_PER_PAGE,
							name: opts?.filters?.id?.value
								? undefined
								: opts?.n?.join(","),
						},
						{ abortSignal },
					);

					return {
						...data,
						pagination: {
							cursor: data.pagination.cursor || null,
						},
						actors: data.actors.map((actor) =>
							transformActor(actor),
						),
					};
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.actors.length < ACTORS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
				},
			});
		},
		buildsQueryOptions() {
			return infiniteQueryOptions({
				...def.buildsQueryOptions(),
				queryKey: [namespace, "builds"],
				enabled: true,
				queryFn: async ({ signal: abortSignal, pageParam }) => {
					const data = await client.actorsListNames(
						{
							namespace,
							cursor: pageParam ?? undefined,
							limit: ACTORS_PER_PAGE,
						},
						{ abortSignal },
					);

					return {
						pagination: data.pagination,
						builds: Object.keys(data.names)
							.sort()
							.map((build) => ({
								id: build,
								name: build,
							})),
					};
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.builds.length < ACTORS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
				},
			});
		},
		createActorMutationOptions() {
			return {
				...def.createActorMutationOptions(),
				mutationKey: [namespace, "actors"],
				mutationFn: async (data) => {
					const response = await client.actorsCreate({
						namespace,
						name: data.name,
						key: data.key,
						crashPolicy: data.crashPolicy,
						runnerNameSelector: data.runnerNameSelector,
						input: JSON.stringify(data.input),
					});

					return response.actor.actorId;
				},
				onSuccess: () => {},
			};
		},
		actorDestroyMutationOptions(actorId) {
			return {
				...def.actorDestroyMutationOptions(actorId),
				mutationFn: async () => {
					await client.actorsDelete(actorId);
				},
			};
		},
	} satisfies ManagerContext;
};

export const NamespaceNameId = z.string().brand();
export type NamespaceNameId = z.infer<typeof NamespaceNameId>;

export const runnersQueryOptions = (opts: { namespace: NamespaceNameId }) => {
	return infiniteQueryOptions({
		queryKey: [opts.namespace, "runners"],
		initialPageParam: undefined as string | undefined,
		queryFn: async ({ pageParam, signal: abortSignal }) => {
			const data = await client.runners.list(
				{
					namespace: opts.namespace,
					cursor: pageParam ?? undefined,
					limit: ACTORS_PER_PAGE,
				},
				{ abortSignal },
			);
			return data;
		},
		getNextPageParam: (lastPage) => {
			if (lastPage.runners.length < ACTORS_PER_PAGE) {
				return undefined;
			}
			return lastPage.pagination.cursor;
		},
		select: (data) => data.pages.flatMap((page) => page.runners),
	});
};

export const runnerQueryOptions = (opts: {
	namespace: NamespaceNameId;
	runnerId: string;
}) => {
	return queryOptions({
		queryKey: [opts.namespace, "runner", opts.runnerId],
		enabled: !!opts.runnerId,
		queryFn: async ({ signal: abortSignal }) => {
			const data = await client.runners.get(
				opts.runnerId,
				{ namespace: opts.namespace },
				{
					abortSignal,
				},
			);
			return data.runner;
		},
	});
};

export const runnerNamesQueryOptions = (opts: {
	namespace: NamespaceNameId;
}) => {
	return infiniteQueryOptions({
		queryKey: [opts.namespace, "runner", "names"],
		initialPageParam: undefined as string | undefined,
		queryFn: async ({ signal: abortSignal, pageParam }) => {
			const data = await client.runners.listNames(
				{
					namespace: opts.namespace,
					cursor: pageParam ?? undefined,
					limit: ACTORS_PER_PAGE,
				},
				{
					abortSignal,
				},
			);
			return data;
		},
		getNextPageParam: (lastPage) => {
			if (lastPage.names.length < ACTORS_PER_PAGE) {
				return undefined;
			}
			return lastPage.pagination.cursor;
		},
		select: (data) => data.pages.flatMap((page) => page.names),
	});
};

export const namespacesQueryOptions = () => {
	return infiniteQueryOptions({
		queryKey: ["namespaces"],
		initialPageParam: undefined as string | undefined,
		queryFn: async ({ pageParam, signal: abortSignal }) => {
			const data = await client.namespaces.list(
				{
					limit: ACTORS_PER_PAGE,
					cursor: pageParam ?? undefined,
				},
				{ abortSignal },
			);
			return data;
		},
		getNextPageParam: (lastPage) => {
			if (lastPage.namespaces.length < ACTORS_PER_PAGE) {
				return undefined;
			}
			return lastPage.pagination.cursor;
		},
		select: (data) => data.pages.flatMap((page) => page.namespaces),
	});
};

export const namespaceQueryOptions = (
	namespace: NamespaceNameId | undefined,
) => {
	return queryOptions({
		queryKey: ["namespace", namespace],
		enabled: !!namespace,
		queryFn: namespace
			? async ({ signal: abortSignal }) => {
					const data = await client.namespaces.get(namespace, {
						abortSignal,
					});
					return data.namespace;
				}
			: skipToken,
	});
};

function transformActor(a: Rivet.Actor): Actor {
	return {
		id: a.actorId as ActorId,
		name: a.name,
		key: a.key ? a.key : undefined,
		connectableAt: a.connectableTs
			? new Date(a.connectableTs).toISOString()
			: undefined,
		region: a.datacenter,
		createdAt: new Date(a.createTs).toISOString(),
		startedAt: a.startTs ? new Date(a.startTs).toISOString() : undefined,
		destroyedAt: a.destroyTs
			? new Date(a.destroyTs).toISOString()
			: undefined,
		sleepingAt: a.sleepTs ? new Date(a.sleepTs).toISOString() : undefined,
		pendingAllocationAt: a.pendingAllocationTs
			? new Date(a.pendingAllocationTs).toISOString()
			: undefined,
		crashPolicy: a.crashPolicy as CrashPolicy,
		runner: a.runnerNameSelector,
		features: [
			ActorFeature.Config,
			ActorFeature.Connections,
			ActorFeature.State,
			ActorFeature.Console,
			ActorFeature.Database,
			ActorFeature.EventsMonitoring,
		],
	};
}
