import type { Rivet } from "@rivetkit/engine-api-full";
import {
	infiniteQueryOptions,
	mutationOptions,
	type QueryKey,
	queryOptions,
} from "@tanstack/react-query";
import { z } from "zod";
import { type ActorId, getActorStatus } from "@/components/actors";
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
		limit: z
			.number()
			.optional()
			.catch(() => RECORDS_PER_PAGE),
	})
	.optional();
export type ActorQueryOptions = z.infer<typeof ActorQueryOptionsSchema>;

export const RECORDS_PER_PAGE = 10;

type CreateActor = Omit<Rivet.ActorsCreateRequest, "namespace">;

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
				// biome-ignore lint/correctness/noUnreachable: stub
				return {} as Rivet.ActorsListResponse;
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

				return lastPage.actors[lastPage.actors.length - 1].actorId;
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
				// biome-ignore lint/correctness/noUnreachable: stub
				return {} as Rivet.ActorsListNamesResponse;
			},
			getNextPageParam: () => {
				return undefined;
			},
			select: (data) => {
				// Flatten the paginated responses into a single list of builds
				return data.pages.flatMap((page) =>
					Object.entries(page.names).map(([id, name]) => ({
						id,
						name,
					})),
				);
			},
		});
	},

	buildsCountQueryOptions() {
		return infiniteQueryOptions({
			...this.buildsQueryOptions(),
			select: (data) => {
				return data.pages.reduce((acc, page) => {
					return acc + Object.keys(page.names).length;
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
					page.actors.map((actor) => actor.actorId),
				);
			},
		});
	},

	actorsListPaginationQueryOptions(opts: ActorQueryOptions) {
		return infiniteQueryOptions({
			...this.actorsQueryOptions(opts),
			select: (data) => {
				return data.pages.flatMap((page) =>
					page.actors.map((actor) => actor.actorId),
				).length;
			},
		});
	},

	// #region Actor Queries
	actorQueryOptions(actorId: ActorId) {
		return queryOptions({
			refetchInterval: 5000,
			queryFn: async () => {
				return {} as Rivet.Actor;
			},
			queryKey: ["actor", actorId] as QueryKey,
		});
	},

	actorDestroyedAtQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) =>
				data.destroyTs ? new Date(data.destroyTs) : null,
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
			select: ({ rescheduleTs, error }) => ({
				rescheduleTs,
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

	actorGeneralQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => ({
				keys: data.key,
				createTs: data.createTs ? new Date(data.createTs) : null,
				destroyTs: data.destroyTs ? new Date(data.destroyTs) : null,
				connectableTs: data.connectableTs
					? new Date(data.connectableTs)
					: null,
				pendingAllocationTs: data.pendingAllocationTs
					? new Date(data.pendingAllocationTs)
					: null,
				sleepTs: data.sleepTs ? new Date(data.sleepTs) : null,
				datacenter: data.datacenter,
				runner: data.runnerNameSelector,
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
		return mutationOptions({
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
		});
	},
	actorLogsQueryOptions(actorId: ActorId) {
		return infiniteQueryOptions({
			queryKey: ["actor", actorId, "logs"] as QueryKey,
			initialPageParam: null as string | null,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
				return [];
			},
			getNextPageParam: () => null,
		});
	},
	actorWorkerQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorQueryOptions(actorId),
			select: (data) => ({
				name: data.name ?? null,
				endpoint: this.endpoint ?? null,
				destroyedAt: data.destroyTs ? new Date(data.destroyTs) : null,
				runner: data.runnerNameSelector ?? undefined,
				sleepingAt: data.sleepTs ? new Date(data.sleepTs) : null,
				startedAt: data.startTs ? new Date(data.startTs) : null,
			}),
		});
	},
	// #endregion
	datacentersQueryOptions() {
		return infiniteQueryOptions({
			queryKey: ["actor", "regions"] as QueryKey,
			initialPageParam: null as string | null,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
				return {} as Rivet.DatacentersListResponse;
			},
			getNextPageParam: () => null,
			select: (data) => data.pages.flatMap((page) => page.datacenters),
		});
	},
	datacenterQueryOptions(regionId: string | undefined) {
		return queryOptions({
			queryKey: ["actor", "region", regionId] as QueryKey,
			enabled: !!regionId,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
				return {} as Rivet.Datacenter;
			},
		});
	},
	createActorMutationOptions() {
		return mutationOptions({
			mutationKey: ["createActor"] as QueryKey,
			mutationFn: async (_: CreateActor) => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
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
		});
	},

	metadataQueryOptions() {
		return queryOptions({
			queryKey: ["metadata"] as QueryKey,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
				return {} as Rivet.MetadataGetResponse;
			},
		});
	},

	actorInspectorTokenQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: 1000,
			gcTime: 1000,
			queryKey: ["tokens", actorId, "inspector"] as QueryKey,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
				return "" as string;
			},
		});
	},

	statusQueryOptions() {
		return queryOptions({
			queryKey: ["status"] as QueryKey,
			queryFn: async () => {
				throw new Error("Not implemented");
				// biome-ignore lint/correctness/noUnreachable: stub
				return true as boolean;
			},
			enabled: false,
			refetchInterval: 5000,
			meta: {
				statusCheck: true,
			},
		});
	},
};

export type DefaultDataProvider = typeof defaultContext;

export function createDefaultGlobalContext(): DefaultDataProvider {
	return defaultContext;
}
