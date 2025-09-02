import type { CreateActor as InspectorCreateActor } from "@rivetkit/core/inspector";
import {
	infiniteQueryOptions,
	type MutationOptions,
	type QueryClient,
	queryOptions,
} from "@tanstack/react-query";
import { createContext, useContext } from "react";
import { z } from "zod";
import { queryClient } from "@/queries/global";
import {
	type Actor,
	type ActorId,
	type ActorLogEntry,
	type ActorMetrics,
	type Build,
	type CrashPolicy,
	getActorStatus,
	type Region,
} from "./queries";

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

export const ACTORS_PER_PAGE = 10;

type PaginatedResponse<T, Field extends string> = {
	pagination: { cursor?: string | null };
} & Record<Field, T[]>;

type PaginatedActorResponse = PaginatedResponse<Actor, "actors">;
type PaginatedBuildsResponse = PaginatedResponse<Build, "builds">;
type PaginatedRegionsResponse = PaginatedResponse<Region, "regions">;

type CreateActor = Omit<InspectorCreateActor, "keys" | "key"> & {
	runnerNameSelector: string;
	key: string;
	crashPolicy: CrashPolicy;
};

const defaultContext = {
	endpoint: "",
	features: {
		canCreateActors: true,
		canDeleteActors: false,
	},
	actorsQueryOptions(opts: ActorQueryOptions) {
		return infiniteQueryOptions({
			queryKey: ["actors", opts],
			initialPageParam: undefined as string | undefined,
			enabled: false,
			refetchInterval: 2000,
			queryFn: async () => {
				return {} as PaginatedActorResponse;
			},
			getNextPageParam: (lastPage) => {
				if (lastPage.pagination.cursor) {
					return lastPage.pagination.cursor;
				}

				if (
					!lastPage ||
					lastPage.actors.length === 0 ||
					lastPage.actors.length < ACTORS_PER_PAGE
				) {
					return undefined;
				}

				return lastPage.actors[lastPage.actors.length - 1].id;
			},
		});
	},

	buildsQueryOptions() {
		return infiniteQueryOptions({
			queryKey: ["actors", "builds"],
			enabled: false,
			initialPageParam: undefined as string | undefined,
			refetchInterval: 2000,
			queryFn: async () => {
				return {} as PaginatedBuildsResponse;
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
			queryKey: ["actor", actorId],
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
				crashPolicy: data.crashPolicy,
			}),
		});
	},
	actorBuildQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "build"],
			queryFn: async () => {
				return {} as Build;
			},
			enabled: false,
		});
	},
	actorMetricsQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "metrics"],
			queryFn: async () => {
				return {} as ActorMetrics;
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
	actorDestroyMutationOptions(actorId: ActorId) {
		return {
			mutationKey: ["actor", actorId, "destroy"],
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
			queryKey: ["actor", actorId, "logs"],
			initialPageParam: null as string | null,
			queryFn: async () => {
				return [] as ActorLogEntry[];
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
			select: ({ runtime, lifecycle, resources, tags }) => ({
				runtime,
				lifecycle,
				resources,
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
				startedAt: data.startedAt ? new Date(data.startedAt) : null,
			}),
		});
	},
	// #endregion
	regionsQueryOptions() {
		return infiniteQueryOptions({
			queryKey: ["actor", "regions"],
			initialPageParam: null as string | null,
			queryFn: async () => {
				return {} as PaginatedRegionsResponse;
			},
			getNextPageParam: () => null,
			select: (data) => data.pages.flatMap((page) => page.regions),
		});
	},
	regionQueryOptions(regionId: string) {
		return queryOptions({
			queryKey: ["actor", "region", regionId],
			enabled: !!regionId,
			queryFn: async () => {
				return {} as Region;
			},
		});
	},
	managerStatusQueryOptions() {
		return queryOptions({
			queryKey: ["managerStatus"],
			refetchInterval: 1000,
			enabled: false,
			retry: 0,
			queryFn: async () => {
				return false as boolean;
			},
		});
	},
	createActorMutationOptions() {
		return {
			mutationKey: ["createActor"],
			mutationFn: async (_: CreateActor) => {
				return "";
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
		};
	},
};

export type ManagerContext = typeof defaultContext;

export function createDefaultManagerContext(): ManagerContext {
	return defaultContext;
}

const ManagerContext = createContext({} as ManagerContext);

export const useManager = () => useContext(ManagerContext);

export const ManagerProvider = ManagerContext.Provider;
