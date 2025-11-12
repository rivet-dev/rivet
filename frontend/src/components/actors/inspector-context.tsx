import {
	mutationOptions,
	type QueryClient,
	queryOptions,
	useQueryClient,
} from "@tanstack/react-query";
import * as cbor from "cbor-x";
import { createContext, useContext, useMemo, useRef } from "react";
import type ReconnectingWebSocket from "reconnectingwebsocket";
import {
	type Connection,
	type Event,
	type ToServer,
	TO_CLIENT_VERSIONED as toClient,
	TO_SERVER_VERSIONED as toServer,
} from "rivetkit/inspector";
import { toast } from "sonner";
import { match } from "ts-pattern";
import { type ConnectionStatus, useWebSocket } from "../hooks/use-websocket";
import type { ActorId } from "./queries";

export const actorInspectorQueriesKeys = {
	actorState: (actorId: ActorId) => ["actor", actorId, "state"] as const,
	actorIsStateEnabled: (actorId: ActorId) =>
		["actor", actorId, "is-state-enabled"] as const,
	actorConnections: (actorId: ActorId) =>
		["actor", actorId, "connections"] as const,
	actorDatabase: (actorId: ActorId) =>
		["actor", actorId, "database"] as const,
	actorEvents: (actorId: ActorId) => ["actor", actorId, "events"] as const,
	actorRpcs: (actorId: ActorId) => ["actor", actorId, "rpcs"] as const,
	actorClearEvents: (actorId: ActorId) =>
		["actor", actorId, "clear-events"] as const,
	actorWakeUp: (actorId: ActorId) => ["actor", actorId, "wake-up"] as const,
	actorAutoWakeUp: (actorId: ActorId) =>
		["actor", actorId, "auto-wake-up"] as const,
};

interface ActorInspectorApi {
	ping: () => Promise<void>;
	executeAction: (name: string, args: unknown[]) => Promise<unknown>;
	patchState: (state: unknown) => Promise<void>;
	getConnections: () => Promise<Connection[]>;
	getEvents: () => Promise<Event[]>;
	getState: () => Promise<unknown>;
}

export const createDefaultActorInspectorContext = ({
	api,
}: {
	api: ActorInspectorApi;
}) => ({
	api,
	actorStateQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorState(actorId),
			queryFn: () => {
				return api.getState();
			},
		});
	},

	actorIsStateEnabledQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorIsStateEnabled(actorId),
			queryFn: () => {
				return false;
			},
		});
	},

	actorConnectionsQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorConnections(actorId),
			queryFn: () => {
				return api.getConnections();
			},
		});
	},

	actorDatabaseQueryOptions(actorId: ActorId) {
		// TODO: implement
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorDatabase(actorId),
			queryFn: () => {
				return { enabled: false, db: [] } as unknown as {
					enabled: boolean;
					db: {
						table: { name: string; type: string };
						records: number;
					}[];
				};
			},
		});
	},

	actorDatabaseEnabledQueryOptions(actorId: ActorId) {
		// TODO: implement
		return queryOptions({
			staleTime: Infinity,
			...this.actorDatabaseQueryOptions(actorId),
			select: (data) => data.enabled,
		});
	},

	actorDatabaseTablesQueryOptions(actorId: ActorId) {
		// TODO: implement
		return queryOptions({
			...this.actorDatabaseQueryOptions(actorId),
			select: (data) =>
				data.db?.map((table) => ({
					name: table.table.name,
					type: table.table.type,
					records: table.records,
				})) || [],
			notifyOnChangeProps: ["data", "isError", "isLoading"],
		});
	},

	actorDatabaseRowsQueryOptions(actorId: ActorId, table: string) {
		// TODO: implement
		return queryOptions({
			staleTime: Infinity,
			queryKey: [
				...actorInspectorQueriesKeys.actorDatabase(actorId),
				table,
			],
			queryFn: () => {
				return [] as unknown as Record<string, unknown>[];
			},
		});
	},

	actorEventsQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorEvents(actorId),
			queryFn: () => {
				return api.getEvents();
			},
		});
	},

	actorRpcsQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorRpcs(actorId),
			queryFn: () => {
				return [] as string[];
			},
		});
	},

	actorClearEventsMutationOptions(actorId: ActorId) {
		return mutationOptions({
			mutationKey: ["actor", actorId, "clear-events"],
			// TODO:
		});
	},

	actorWakeUpMutationOptions(actorId: ActorId) {
		return mutationOptions({
			mutationKey: ["actor", actorId, "wake-up"],
			// TODO:
		});
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
			queryKey: actorInspectorQueriesKeys.actorAutoWakeUp(actorId),
			retry: false,
			//FIXME:
		});
	},

	actorPingQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "ping"],
			queryFn: async () => {
				try {
					await api.ping();
					return true;
				} catch {
					return false;
				}
			},
			retry: false,
		});
	},

	actorStatePatchMutation(actorId: ActorId) {
		return mutationOptions({
			mutationKey: ["actor", actorId, "state", "patch"],
			mutationFn: async (state: unknown) => {
				return api.patchState(state);
			},
		});
	},
});

export type ActorInspectorContext = ReturnType<
	typeof createDefaultActorInspectorContext
> & { connectionStatus: ConnectionStatus };

const ActorInspectorContext = createContext({} as ActorInspectorContext);

export const useActorInspector = () => useContext(ActorInspectorContext);

export const ActorInspectorProvider = ({
	children,
	actorId,
	credentials,
}: {
	children: React.ReactNode;
	actorId: ActorId;
	credentials: { url: string; inspectorToken: string; token: string };
}) => {
	const protocols = useMemo(
		() =>
			[
				"rivet",
				`rivet_target.actor`,
				`rivet_actor.${actorId}`,
				`rivet_encoding.bare`,
				credentials.token ? `rivet_token.${credentials.token}` : "",
				credentials.inspectorToken
					? `rivet_inspector_token.${credentials.inspectorToken}`
					: "",
			].filter(Boolean),
		[actorId, credentials.token, credentials.inspectorToken],
	);

	const queryClient = useQueryClient();

	const actionsManager = useRef(new ActionsManager());

	const onMessage = useMemo(() => {
		return createMessageHandler({ queryClient, actorId, actionsManager });
	}, [queryClient, actorId]);

	const { sendMessage, reconnect, status } = useWebSocket(
		new URL(`/gateway/${actorId}/inspector/connect`, credentials.url).href,
		protocols,
		{ onMessage },
	);

	const api = useMemo(() => {
		return {
			ping: async () => {
				return reconnect();
			},
			executeAction: async (name, args) => {
				const { id, promise } =
					actionsManager.current.createSuspension<unknown>();

				sendMessage(
					serverMessage({
						body: {
							tag: "ActionRequest",
							val: {
								id: BigInt(id),
								name,
								args: new Uint8Array(cbor.encode(args)).buffer,
							},
						},
					}),
				);

				return promise;
			},

			patchState: async (state) => {
				sendMessage(
					serverMessage({
						body: {
							tag: "PatchStateRequest",
							val: {
								state: new Uint8Array(cbor.encode(state))
									.buffer,
							},
						},
					}),
				);
			},

			getConnections: async () => {
				const { id, promise } =
					actionsManager.current.createSuspension<Connection[]>();

				sendMessage(
					serverMessage({
						body: {
							tag: "ConnectionsRequest",
							val: { id: BigInt(id) },
						},
					}),
				);

				return promise;
			},

			getEvents: async () => {
				const { id, promise } =
					actionsManager.current.createSuspension<Event[]>();

				sendMessage(
					serverMessage({
						body: {
							tag: "EventsRequest",
							val: { id: BigInt(id) },
						},
					}),
				);

				return promise;
			},

			getState: async () => {
				const { id, promise } =
					actionsManager.current.createSuspension<unknown>();

				sendMessage(
					serverMessage({
						body: {
							tag: "StateRequest",
							val: { id: BigInt(id) },
						},
					}),
				);

				return promise;
			},
		} satisfies ActorInspectorApi;
	}, [sendMessage, reconnect]);

	const value = useMemo(() => {
		return {
			connectionStatus: status,
			...createDefaultActorInspectorContext({
				api,
			}),
		};
	}, [api, status]);

	return (
		<ActorInspectorContext.Provider value={value}>
			{children}
		</ActorInspectorContext.Provider>
	);
};

const createMessageHandler =
	({
		queryClient,
		actorId,
		actionsManager,
	}: {
		queryClient: QueryClient;
		actorId: ActorId;
		actionsManager: React.RefObject<ActionsManager>;
	}) =>
	async (e: ReconnectingWebSocket.MessageEvent) => {
		const message = toClient.deserializeWithEmbeddedVersion(
			new Uint8Array(await e.data.arrayBuffer()),
		);

		match(message.body)
			.with({ tag: "Init" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorState(actorId),
					transformState(body.val.state),
				);

				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorConnections(actorId),
					transformConnections(body.val.connections),
				);

				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorEvents(actorId),
					transformEvents(body.val.events),
				);

				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorIsStateEnabled(actorId),
					body.val.isStateEnabled,
				);
			})
			.with({ tag: "ConnectionsResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolveSuspension(
					Number(rid),
					transformConnections(body.val.connections),
				);
			})
			.with({ tag: "EventsResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolveSuspension(
					Number(rid),
					transformEvents(body.val.events),
				);
			})
			.with({ tag: "StateResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolveSuspension(
					Number(rid),
					cbor.decode(new Uint8Array(body.val.state)),
				);
			})
			.with({ tag: "ActionResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolveSuspension(
					Number(rid),
					cbor.decode(new Uint8Array(body.val.output)),
				);
			})
			.with({ tag: "ConnectionsUpdated" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorConnections(actorId),
					transformConnections(body.val.connections),
				);
			})
			.with({ tag: "StateUpdated" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorState(actorId),
					transformState(body.val.state),
				);
			})
			.with({ tag: "EventsUpdated" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorEvents(actorId),
					transformEvents(body.val.events),
				);
			})
			.with({ tag: "Error" }, (body) => {
				toast.error(`Inspector error: ${body.val.message}`);
			})
			.exhaustive();
	};

function transformEvents(events: readonly Event[]) {
	return events.map((event) => {
		const base = {
			...event,
			timestamp: new Date(Number(event.timestamp)),
		};

		return match(event.body)
			.with({ tag: "FiredEvent" }, (body) => ({
				...base,
				body: {
					...body,
					val: {
						...body.val,
						args: cbor.decode(new Uint8Array(body.val.args)),
					},
				},
			}))
			.with({ tag: "ActionEvent" }, (body) => ({
				...base,
				body: {
					...body,
					val: {
						...body.val,
						args: cbor.decode(new Uint8Array(body.val.args)),
					},
				},
			}))
			.with({ tag: "BroadcastEvent" }, (body) => ({
				...base,
				body: {
					...body,
					val: {
						...body.val,
						args: cbor.decode(new Uint8Array(body.val.args)),
					},
				},
			}))
			.with({ tag: "SubscribeEvent" }, (body) => ({
				...base,
				body: {
					...body,
				},
			}))
			.with({ tag: "UnSubscribeEvent" }, (body) => ({
				...base,
				body: {
					...body,
				},
			}))
			.exhaustive();
	});
}

function transformConnections(connections: readonly Connection[]) {
	return connections.map((connection) => ({
		...connection,
		details: cbor.decode(new Uint8Array(connection.details)),
	}));
}

function transformState(state: ArrayBuffer) {
	return cbor.decode(new Uint8Array(state));
}

function serverMessage(data: ToServer) {
	return toServer.serializeWithEmbeddedVersion(data);
}

class ActionsManager {
	private suspensions = new Map<number, PromiseWithResolvers<any>>();

	private nextId = 1;

	createSuspension<T = void>(): { id: number; promise: Promise<T> } {
		const id = this.nextId++;
		const { promise, resolve, reject } = Promise.withResolvers<T>();
		this.suspensions.set(id, { promise, resolve, reject });
		return { id, promise };
	}

	resolveSuspension(id: number, value?: unknown) {
		const suspension = this.suspensions.get(id);
		if (suspension) {
			suspension.resolve(value);
			this.suspensions.delete(id);
		}
	}
}
