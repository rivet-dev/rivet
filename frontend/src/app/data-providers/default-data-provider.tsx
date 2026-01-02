import {
	infiniteQueryOptions,
	type MutationOptions,
	mutationOptions,
	type QueryKey,
	queryOptions,
	UseInfiniteQueryOptions,
} from "@tanstack/react-query";
import type {
	ActorId,
	ActorLogEntry,
	CreateActor as InspectorCreateActor,
} from "rivetkit/inspector";
import { z } from "zod";
import {
	type Actor,
	type ActorMetrics,
	type Build,
	type CrashPolicy,
	getActorStatus,
	type Region,
} from "@/components/actors";
import { queryClient } from "@/queries/global";

export const ActorQueryOptionsSchema = z
	.object({
		filters: z
			.object({
				showDestroyed: z
					.object({ value: z.array(z.string()) })
					.optional()
					.catch(() => ({ value: ["false"] })),
				id: z
					.object({
						value: z.array(z.string()).optional(),
					})
					.optional(),
				key: z
					.object({
						value: z.array(z.string()).optional(),
					})
					.optional(),
			})
			.optional()
			.catch(() => ({})),
		n: z
			.array(z.string())
			.optional()
			.catch(() => []),
	})
	.optional();
export type ActorQueryOptions = z.infer<typeof ActorQueryOptionsSchema>;

export const RECORDS_PER_PAGE = 10;

type PaginatedResponse<T, Field extends string> = {
	pagination: { cursor?: string };
} & Record<Field, T[]>;

type PaginatedActorResponse = PaginatedResponse<Actor, "actors">;
type PaginatedBuildsResponse = PaginatedResponse<Build, "builds">;
type PaginatedRegionsResponse = PaginatedResponse<Region, "regions">;

type CreateActor = Omit<InspectorCreateActor, "keys" | "key"> & {
	runnerNameSelector: string;
	key: string;
	crashPolicy: CrashPolicy;
	datacenter?: string;
};

const defaultContext = {
	endpoint: "",
	features: {
		canCreateActors: true,
		canDeleteActors: false,
	},
	actorsQueryOptions(opts: ActorQueryOptions) {
		return infiniteQueryOptions({
			queryKey: ["actors", opts] as QueryKey,
			initialPageParam: undefined as string | undefined,
			enabled: false,
			refetchInterval: 2000,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
			getNextPageParam: (lastPage) => {
				if (lastPage.pagination.cursor) {
					return lastPage.pagination.cursor;
				}

				if (
					!lastPage ||
					lastPage.actors.length === 0 ||
					lastPage.actors.length < RECORDS_PER_PAGE
				) {
					return undefined;
				}

				return lastPage.actors[lastPage.actors.length - 1].id;
			},
		});
	},

	buildsQueryOptions() {
		return infiniteQueryOptions({
			queryKey: ["actors", "builds"] as QueryKey,
			enabled: false,
			initialPageParam: undefined as string | undefined,
			refetchInterval: 2000,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
			getNextPageParam: () => {
				return undefined;
			},
			select: (data) => {
				return data.pages.flatMap((page) => page.builds);
			},
		});
	},

	buildsCountQueryOptions() {
		return infiniteQueryOptions({
			...this.buildsQueryOptions(),
			select: (data) => {
				return data.pages.reduce((acc, page) => {
					return acc + page.builds.length;
				}, 0);
			},
		});
	},

	actorsListQueryOptions(opts: ActorQueryOptions) {
		return infiniteQueryOptions({
			...this.actorsQueryOptions(opts),
			enabled: (opts?.n || []).length > 0,
			refetchInterval: 5000,
			select: (data) => {
				return data.pages.flatMap((page) =>
					page.actors.map((actor) => actor.id),
				);
			},
		});
	},

	actorsListPaginationQueryOptions(opts: ActorQueryOptions) {
		return infiniteQueryOptions({
			...this.actorsQueryOptions(opts),
			select: (data) => {
				return data.pages.flatMap((page) =>
					page.actors.map((actor) => actor.id),
				).length;
			},
		});
	},

	// #region Actor Queries
	actorQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryFn: async () => {
				return {} as Actor;
			},
			queryKey: ["actor", actorId] as QueryKey,
		});
	},

	actorRegionQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => data.region,
		});
	},

	actorDestroyedAtQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) =>
				data.destroyedAt ? new Date(data.destroyedAt) : null,
		});
	},

	actorStatusQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => getActorStatus(data),
		});
	},

	actorStatusAdditionalInfoQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: ({ rescheduleAt, error }) => ({
				rescheduleAt,
				error,
			}),
		});
	},

	actorErrorQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => data.error,
		});
	},

	actorFeaturesQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => data.features ?? [],
		});
	},

	actorGeneralQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => ({
				tags: data.tags,
				keys: data.key,
				createdAt: data.createdAt ? new Date(data.createdAt) : null,
				destroyedAt: data.destroyedAt
					? new Date(data.destroyedAt)
					: null,
				connectableAt: data.connectableAt
					? new Date(data.connectableAt)
					: null,
				pendingAllocationAt: data.pendingAllocationAt
					? new Date(data.pendingAllocationAt)
					: null,
				sleepingAt: data.sleepingAt ? new Date(data.sleepingAt) : null,
				region: data.region,
				runner: data.runner,
				crashPolicy: data.crashPolicy,
			}),
		});
	},
	actorBuildQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "build"] as QueryKey,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
			enabled: false,
		});
	},
	actorMetricsQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "metrics"] as QueryKey,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
			enabled: false,
		});
	},
	actorKeysQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => data.key,
		});
	},
	actorDatacenterQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => data.datacenter ?? null,
		});
	},
	actorDestroyMutationOptions(actorId: ActorId) {
		return {
			mutationKey: ["actor", actorId, "destroy"] as QueryKey,
			mutationFn: async () => {
				return;
			},
			onSuccess: () => {
				const keys = this.actorQueryOptions(actorId).queryKey.filter(
					(k) => typeof k === "string",
				);
				queryClient.invalidateQueries({
					predicate: (query) => {
						return keys.every((k) => query.queryKey.includes(k));
					},
				});
			},
		} satisfies MutationOptions;
	},
	actorLogsQueryOptions(actorId: ActorId) {
		return infiniteQueryOptions({
			queryKey: ["actor", actorId, "logs"] as QueryKey,
			initialPageParam: null as string | null,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
			getNextPageParam: () => null,
		});
	},
	actorNetworkQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => data.network,
		});
	},
	actorNetworkPortsQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorNetworkQueryOptions(actorId),
			select: (data) => data.network?.ports,
		});
	},
	actorRuntimeQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: ({ runtime, lifecycle, tags }) => ({
				runtime,
				lifecycle,
				tags,
			}),
		});
	},
	actorWorkerQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => ({
				features: data.features ?? [],
				name: data.name ?? null,
				endpoint: this.endpoint ?? null,
				destroyedAt: data.destroyedAt
					? new Date(data.destroyedAt)
					: null,
				runner: data.runner ?? undefined,
				sleepingAt: data.sleepingAt ? new Date(data.sleepingAt) : null,
				startedAt: data.startedAt ? new Date(data.startedAt) : null,
			}),
		});
	},
	// #endregion
	regionsQueryOptions() {
		return infiniteQueryOptions({
			queryKey: ["actor", "regions"] as QueryKey,
			initialPageParam: null as string | null,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: <explanation>
				return {} as PaginatedRegionsResponse;
			},
			getNextPageParam: () => null,
			select: (data) => data.pages.flatMap((page) => page.regions),
		});
	},
	regionQueryOptions(regionId: string | undefined) {
		return queryOptions({
			queryKey: ["actor", "region", regionId] as QueryKey,
			enabled: !!regionId,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
		});
	},
	statusQueryOptions() {
		return queryOptions({
			queryKey: ["status"] as QueryKey,
			refetchInterval: 1000,
			enabled: false,
			retry: 0,
			queryFn: async () => {
				throw new Error("Not implemented");
			},
		});
	},
	createActorMutationOptions() {
		return mutationOptions({
			mutationKey: ["createActor"] as QueryKey,
			mutationFn: async (_: CreateActor) => {
				throw new Error("Not implemented");
			},
			onSuccess: () => {
				const keys = this.actorsQueryOptions({}).queryKey.filter(
					(k) => typeof k === "string",
				);
				queryClient.invalidateQueries({
					predicate: (query) => {
						return keys.every((k) => query.queryKey.includes(k));
					},
				});
			},
		});
	},
};

export type DefaultDataProvider = typeof defaultContext;

export function createDefaultGlobalContext(): DefaultDataProvider {
	return defaultContext;
}
