import {
	mutationOptions,
	type QueryClient,
	queryOptions,
	useQuery,
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
import z from "zod";
import { type ConnectionStatus, useWebSocket } from "../hooks/use-websocket";
import { useActorInspectorData } from "./hooks/use-actor-inspector-data";
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
};

interface ActorInspectorApi {
	ping: () => Promise<void>;
	executeAction: (name: string, args: unknown[]) => Promise<unknown>;
	patchState: (state: unknown) => Promise<void>;
	getConnections: () => Promise<Connection[]>;
	getEvents: () => Promise<TransformedInspectorEvent[]>;
	getState: () => Promise<{ isEnabled: boolean; state: unknown }>;
	getRpcs: () => Promise<string[]>;
	clearEvents: () => Promise<void>;
	getMetadata: () => Promise<{ version: string }>;
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
				return api.getRpcs();
			},
		});
	},

	actorClearEventsMutationOptions(actorId: ActorId) {
		return mutationOptions({
			mutationKey: ["actor", actorId, "clear-events"],
			mutationFn: async () => {
				return api.clearEvents();
			},
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

	actorMetadataQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "metadata"],
			retry: 0,
			retryDelay: 5_000,
			refetchInterval: 5_000,
			queryFn: async () => {
				return api.getMetadata();
			},
		});
	},
});

const computeActorUrl = ({ url, actorId }: { url: string; actorId: ActorId }) =>
	new URL(`/gateway/${actorId}`, url).href;

export const actorWakeUpMutationOptions = () =>
	mutationOptions({
		mutationKey: ["actor", "wake-up"],
		mutationFn: async ({
			actorId,
			credentials,
		}: {
			actorId: ActorId;
			credentials: { url: string; token: string };
		}) => {
			const response = await fetch(
				new URL(
					`${computeActorUrl({ ...credentials, actorId })}/health`,
				).href,
				{
					headers: {
						"X-Rivet-Target": "actor",
						"X-Rivet-Actor": actorId,
						"x-rivet-token": credentials.token,
					},
				},
			);

			return await response.text();
		},
	});

export const actorWakeUpQueryOptions = ({
	actorId,
	credentials,
}: {
	actorId: ActorId;
	credentials: { url: string; token: string };
}) =>
	queryOptions({
		queryKey: ["actor", actorId, "wake-up"],
		queryFn: async () => {
			return actorWakeUpMutationOptions().mutationFn?.({
				actorId,
				credentials,
			});
		},
	});

const getActorMetadata = async ({
	actorId,
	credentials,
}: {
	actorId: ActorId;
	credentials: { url: string; token: string };
}) => {
	const response = await fetch(
		new URL(`${computeActorUrl({ ...credentials, actorId })}/metadata`)
			.href,
		{
			headers: {
				"X-Rivet-Target": "actor",
				"X-Rivet-Actor": actorId,
				"x-rivet-token": credentials.token,
			},
		},
	);

	if (!response.ok) {
		throw new Error(
			`Failed to fetch actor metadata: ${response.statusText}`,
		);
	}
	return z.object({ version: z.string() }).parse(await response.json());
};

export const actorMetadataQueryOptions = ({
	actorId,
	credentials,
}: {
	actorId: ActorId;
	credentials: { url: string; token: string };
}) =>
	queryOptions({
		queryKey: ["actor", actorId, "metadata"],
		retry: 0,
		retryDelay: 5_000,
		refetchInterval: 1_000,
		queryFn: async () => {
			return getActorMetadata({ actorId, credentials });
		},
	});

export type ActorInspectorContext = ReturnType<
	typeof createDefaultActorInspectorContext
> & { connectionStatus: ConnectionStatus; isInspectorAvailable: boolean };

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

	const { isSuccess: isActorMetadataSuccess } = useQuery({
		...actorMetadataQueryOptions({ actorId, credentials }),
	});

	const { isSuccess: isActorDataSuccess } = useActorInspectorData(actorId);

	const isInspectorAvailable = isActorMetadataSuccess && isActorDataSuccess;

	const onMessage = useMemo(() => {
		return createMessageHandler({ queryClient, actorId, actionsManager });
	}, [queryClient, actorId]);

	const { sendMessage, reconnect, status } = useWebSocket(
		`${computeActorUrl({ ...credentials, actorId })}/inspector/connect`,
		protocols,
		{ onMessage, enabled: isInspectorAvailable },
	);

	const getActorMetadataProxy = useRef(async () => {
		return getActorMetadata({ actorId, credentials });
	});

	const api = useMemo(() => {
		return {
			ping: async () => {
				return reconnect();
			},
			executeAction: async (name, args) => {
				const { id, promise } =
					actionsManager.current.createResolver<unknown>();

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
					actionsManager.current.createResolver<Connection[]>();

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
					actionsManager.current.createResolver<
						TransformedInspectorEvent[]
					>();

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
				const { id, promise } = actionsManager.current.createResolver<{
					isEnabled: boolean;
					state: unknown;
				}>();

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

			clearEvents: async () => {
				const { id, promise } = actionsManager.current.createResolver();
				sendMessage(
					serverMessage({
						body: {
							tag: "ClearEventsRequest",
							val: { id: BigInt(id) },
						},
					}),
				);
				return promise;
			},

			getRpcs() {
				const { id, promise } =
					actionsManager.current.createResolver<string[]>();

				sendMessage(
					serverMessage({
						body: {
							tag: "RpcsListRequest",
							val: { id: BigInt(id) },
						},
					}),
				);

				return promise;
			},

			getMetadata() {
				return getActorMetadataProxy.current();
			},
		} satisfies ActorInspectorApi;
	}, [sendMessage, reconnect]);

	const value = useMemo(() => {
		return {
			connectionStatus: status,
			isInspectorAvailable,
			...createDefaultActorInspectorContext({
				api,
			}),
		};
	}, [api, status, isInspectorAvailable]);

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
					!body.val.isStateEnabled || body.val.state == null
						? { state: null, isEnabled: body.val.isStateEnabled }
						: {
								state: transformState(body.val.state),
								isEnabled: body.val.isStateEnabled,
							},
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
				actionsManager.current.resolve(
					Number(rid),
					transformConnections(body.val.connections),
				);
			})
			.with({ tag: "EventsResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					transformEvents(body.val.events),
				);
			})
			.with({ tag: "StateResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					body.val.state
						? {
								state: cbor.decode(
									new Uint8Array(body.val.state),
								),
								isEnabled: body.val.isStateEnabled,
							}
						: { state: null, isEnabled: body.val.isStateEnabled },
				);
			})
			.with({ tag: "ActionResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
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
					{ isEnabled: true, state: transformState(body.val.state) },
				);
			})
			.with({ tag: "EventsUpdated" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorEvents(actorId),
					transformEvents(body.val.events),
				);
			})
			.with({ tag: "RpcsListResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(Number(rid), body.val.rpcs);
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

export type TransformedInspectorEvent = ReturnType<
	typeof transformEvents
>[number];

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
	return toServer.serializeWithEmbeddedVersion(data, 1);
}

class ActionsManager {
	private suspensions = new Map<number, PromiseWithResolvers<any>>();

	private nextId = 1;

	createResolver<T = void>(): { id: number; promise: Promise<T> } {
		const id = this.nextId++;
		const { promise, resolve, reject } = Promise.withResolvers<T>();
		this.suspensions.set(id, { promise, resolve, reject });

		// set a timeout to reject the promise if not resolved in time
		setTimeout(() => {
			if (this.suspensions.has(id)) {
				reject(new Error("Action timed out"));
				this.suspensions.delete(id);
			}
		}, 2_000);

		return { id, promise };
	}

	resolve(id: number, value?: unknown) {
		const suspension = this.suspensions.get(id);
		if (suspension) {
			suspension.resolve(value);
			this.suspensions.delete(id);
		}
	}
}
