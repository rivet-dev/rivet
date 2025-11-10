import { queryOptions } from "@tanstack/react-query";
import { createContext, useContext } from "react";
import {
	createActorInspectorClient,
	type RecordedRealtimeEvent,
} from "rivetkit/inspector";
import type { ActorId } from "./queries";

type RequestOptions = Parameters<typeof createActorInspectorClient>[1];

export const createDefaultActorContext = (
	{ hash }: { hash: string } = { hash: `${Date.now()}` },
) => ({
	createActorInspectorFetchConfiguration: async (
		actorId: ActorId | string,
		opts: { auth?: boolean } = { auth: true },
	): Promise<RequestOptions> => ({
		headers: {
			"X-RivetKit-Query": JSON.stringify({
				getForId: { actorId },
			}),
		},
	}),
	createActorInspectorUrl(actorId: ActorId | string) {
		return "http://localhost:6420/registry/actors/inspect";
	},
	async createActorInspector(
		actorId: ActorId | string,
		opts: { auth?: boolean } = { auth: true },
	) {
		return createActorInspectorClient(
			this.createActorInspectorUrl(actorId),
			await this.createActorInspectorFetchConfiguration(actorId, opts),
		);
	},
	actorPingQueryOptions(
		actorId: ActorId,
		opts: { enabled?: boolean; refetchInterval?: number | false } = {},
	) {
		return queryOptions({
			enabled: false,
			refetchInterval: 1000,
			...opts,
			queryKey: [hash, "actor", actorId, "ping"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.ping.$get();
				if (!response.ok) {
					throw response;
				}
				return await response.json();
			},
		});
	},

	actorStateQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			enabled,
			refetchInterval: 1000,
			queryKey: [hash, "actor", actorId, "state"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.state.$get();

				if (!response.ok) {
					throw response;
				}
				return (await response.json()) as {
					enabled: boolean;
					state: unknown;
				};
			},
		});
	},

	actorConnectionsQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			enabled,
			refetchInterval: 1000,
			queryKey: [hash, "actor", actorId, "connections"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.connections.$get();

				if (!response.ok) {
					throw response;
				}
				return await response.json();
			},
		});
	},

	actorDatabaseQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			enabled,
			queryKey: [hash, "actor", actorId, "database"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.db.$get();

				if (!response.ok) {
					throw response;
				}
				return await response.json();
			},
		});
	},

	actorDatabaseEnabledQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			...this.actorDatabaseQueryOptions(actorId, { enabled }),
			select: (data) => data.enabled,
			notifyOnChangeProps: ["data", "isError", "isLoading"],
		});
	},

	actorDatabaseTablesQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			...this.actorDatabaseQueryOptions(actorId, { enabled }),
			select: (data) =>
				data.db?.map((table) => ({
					name: table.table.name,
					type: table.table.type,
					records: table.records,
				})) || [],
			notifyOnChangeProps: ["data", "isError", "isLoading"],
		});
	},

	actorDatabaseRowsQueryOptions(
		actorId: ActorId,
		table: string,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			enabled,
			staleTime: 0,
			gcTime: 5000,
			queryKey: [hash, "actor", actorId, "database", table],
			queryFn: async ({ queryKey: [, , actorId, , table] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.db.$post({
					json: { query: `SELECT * FROM ${table} LIMIT 500` },
				});
				if (!response.ok) {
					throw response;
				}
				return await response.json();
			},
		});
	},

	actorEventsQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			enabled,
			refetchInterval: 1000,
			queryKey: [hash, "actor", actorId, "events"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.events.$get();

				if (!response.ok) {
					throw response;
				}
				return (await response.json()) as {
					events: RecordedRealtimeEvent[];
				};
			},
		});
	},

	actorRpcsQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled: boolean } = { enabled: true },
	) {
		return queryOptions({
			enabled,
			queryKey: [hash, "actor", actorId, "rpcs"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId);
				const response = await client.rpcs.$get();

				if (!response.ok) {
					throw response;
				}
				return await response.json();
			},
		});
	},

	actorClearEventsMutationOptions(actorId: ActorId) {
		return {
			mutationKey: [hash, "actor", actorId, "clear-events"],
			mutationFn: async () => {
				const client = await this.createActorInspector(actorId);
				const response = await client.events.clear.$post();
				if (!response.ok) {
					throw response;
				}
				return await response.json();
			},
		};
	},

	actorWakeUpMutationOptions(actorId: ActorId) {
		return {
			mutationKey: [hash, "actor", actorId, "wake-up"],
			mutationFn: async () => {
				const client = await this.createActorInspector(actorId);
				try {
					await client.ping.$get();
					return true;
				} catch {
					return false;
				}
			},
		};
	},

	actorAutoWakeUpQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled?: boolean } = {},
	) {
		return queryOptions({
			enabled,
			refetchInterval: 1000,
			staleTime: 0,
			gcTime: 0,
			queryKey: [hash, "actor", actorId, "auto-wake-up"],
			queryFn: async ({ queryKey: [, , actorId] }) => {
				const client = await this.createActorInspector(actorId, {
					auth: false,
				});
				try {
					await client.ping.$get();
					return true;
				} catch {
					return false;
				}
			},
			retry: false,
		});
	},
});

export type ActorContext = ReturnType<typeof createDefaultActorContext>;

const ActorContext = createContext({} as ActorContext);

export const useActor = () => useContext(ActorContext);

export const ActorProvider = ActorContext.Provider;
