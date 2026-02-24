import type { ReadRangeOptions, ReadRangeWire } from "@rivetkit/traces";
import { decodeReadRangeWire } from "@rivetkit/traces/encoding";
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
	decodeWorkflowHistoryTransport,
	type QueueStatus,
	type ToServer,
	TO_CLIENT_VERSIONED as toClient,
	TO_SERVER_VERSIONED as toServer,
} from "rivetkit/inspector/client";
import { toast } from "sonner";
import { match } from "ts-pattern";
import z from "zod";
import { type ConnectionStatus, useWebSocket } from "../hooks/use-websocket";
import { useActorInspectorData } from "./hooks/use-actor-inspector-data";
import type { ActorId } from "./queries";
import { transformWorkflowHistory } from "./workflow/transform-workflow-history";
import type { WorkflowHistory } from "./workflow/workflow-types";

export const actorInspectorQueriesKeys = {
	actorState: (actorId: ActorId) => ["actor", actorId, "state"] as const,
	actorIsStateEnabled: (actorId: ActorId) =>
		["actor", actorId, "is-state-enabled"] as const,
	actorConnections: (actorId: ActorId) =>
		["actor", actorId, "connections"] as const,
	actorDatabase: (actorId: ActorId) =>
		["actor", actorId, "database"] as const,
	actorRpcs: (actorId: ActorId) => ["actor", actorId, "rpcs"] as const,
	actorTraces: (actorId: ActorId) => ["actor", actorId, "traces"] as const,
	actorQueueStatus: (actorId: ActorId, limit: number) =>
		["actor", actorId, "queue", limit] as const,
	actorQueueSize: (actorId: ActorId) =>
		["actor", actorId, "queue", "size"] as const,
	actorWakeUp: (actorId: ActorId) => ["actor", actorId, "wake-up"] as const,
	actorWorkflowHistory: (actorId: ActorId) =>
		["actor", actorId, "workflow-history"] as const,
	actorIsWorkflowEnabled: (actorId: ActorId) =>
		["actor", actorId, "is-workflow-enabled"] as const,
};

type QueueStatusSummary = {
	size: number;
	maxSize: number;
	truncated: boolean;
	messages: Array<{
		id: string;
		name: string;
		createdAtMs: number;
	}>;
};

export type DatabaseColumn = {
	cid: number;
	name: string;
	type: string;
	notnull: boolean;
	dflt_value: string | null;
	pk: boolean | null;
};

export type DatabaseForeignKey = {
	id: number;
	table: string;
	from: string;
	to: string;
};

export type DatabaseTableInfo = {
	table: { schema: string; name: string; type: string };
	columns: DatabaseColumn[];
	foreignKeys: DatabaseForeignKey[];
	records: number;
};

export type DatabaseSchema = {
	tables: DatabaseTableInfo[];
};

interface ActorInspectorApi {
	ping: () => Promise<void>;
	executeAction: (name: string, args: unknown[]) => Promise<unknown>;
	patchState: (state: unknown) => Promise<void>;
	getConnections: () => Promise<Connection[]>;
	getState: () => Promise<{ isEnabled: boolean; state: unknown }>;
	getRpcs: () => Promise<string[]>;
	getTraces: (options: ReadRangeOptions) => Promise<ReadRangeWire>;
	getQueueStatus: (limit: number) => Promise<QueueStatusSummary>;
	getWorkflowHistory: () => Promise<{
		history: WorkflowHistory | null;
		isEnabled: boolean;
	}>;
	getDatabaseSchema: () => Promise<DatabaseSchema>;
	getDatabaseTableRows: (
		table: string,
		limit: number,
		offset: number,
	) => Promise<unknown[]>;
	getMetadata: () => Promise<{ version: string }>;
}

type FeatureSupport = {
	supported: boolean;
	minVersion: string;
	currentVersion?: string;
	message: string;
};

const MIN_RIVETKIT_VERSION_TRACES = "2.0.40";
const MIN_RIVETKIT_VERSION_QUEUE = "2.0.40";
const MIN_RIVETKIT_VERSION_DATABASE = "2.0.42";
const INSPECTOR_ERROR_EVENTS_DROPPED = "inspector.events_dropped";

function parseSemver(version?: string) {
	if (!version) {
		return null;
	}
	const match = version.match(/^(\d+)\.(\d+)\.(\d+)/);
	if (!match) {
		return null;
	}
	return {
		major: Number(match[1]),
		minor: Number(match[2]),
		patch: Number(match[3]),
	};
}

function compareSemver(
	a: { major: number; minor: number; patch: number },
	b: { major: number; minor: number; patch: number },
) {
	if (a.major !== b.major) {
		return a.major - b.major;
	}
	if (a.minor !== b.minor) {
		return a.minor - b.minor;
	}
	return a.patch - b.patch;
}

function isVersionAtLeast(version: string | undefined, minVersion: string) {
	const parsed = parseSemver(version);
	const minParsed = parseSemver(minVersion);
	if (!parsed || !minParsed) {
		return false;
	}
	return compareSemver(parsed, minParsed) >= 0;
}

function buildFeatureSupport(
	currentVersion: string | undefined,
	minVersion: string,
	label: string,
): FeatureSupport {
	const supported = isVersionAtLeast(currentVersion, minVersion);
	if (!currentVersion) {
		return {
			supported: false,
			minVersion,
			currentVersion,
			message: `${label} requires RivetKit ${minVersion}+. Please upgrade.`,
		};
	}
	return {
		supported,
		minVersion,
		currentVersion,
		message: supported
			? ""
			: `${label} requires RivetKit ${minVersion}+ (current ${currentVersion}). Please upgrade.`,
	};
}

function getInspectorProtocolVersion(version: string | undefined) {
	const parsed = parseSemver(version);
	if (!parsed) {
		return 2;
	}
	if (isVersionAtLeast(version, MIN_RIVETKIT_VERSION_DATABASE)) {
		return 3;
	}
	if (parsed.major >= 2) {
		return 2;
	}
	return 1;
}

function normalizeQueueStatus(status: QueueStatus): QueueStatusSummary {
	return {
		size: Number(status.size),
		maxSize: Number(status.maxSize),
		truncated: status.truncated,
		messages: status.messages.map((message) => ({
			id: message.id.toString(),
			name: message.name,
			createdAtMs: Number(message.createdAtMs),
		})),
	};
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
		return queryOptions({
			staleTime: 0,
			queryKey: actorInspectorQueriesKeys.actorDatabase(actorId),
			queryFn: () => {
				return api.getDatabaseSchema();
			},
		});
	},

	actorDatabaseEnabledQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: [
				...actorInspectorQueriesKeys.actorDatabase(actorId),
				"enabled",
			],
			queryFn: () => new Promise<boolean>(() => {}),
		});
	},

	actorDatabaseTablesQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorDatabaseQueryOptions(actorId),
			select: (data) =>
				data.tables?.map((table) => ({
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
		page: number,
		pageSize = 100,
	) {
		return queryOptions({
			staleTime: 0,
			gcTime: 5000,
			queryKey: [
				...actorInspectorQueriesKeys.actorDatabase(actorId),
				table,
				page,
				pageSize,
			],
			queryFn: () => {
				return api.getDatabaseTableRows(
					table,
					pageSize,
					page * pageSize,
				);
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

	actorTracesQueryOptions(actorId: ActorId, options: ReadRangeOptions) {
		return queryOptions({
			staleTime: 0,
			queryKey: [
				...actorInspectorQueriesKeys.actorTraces(actorId),
				options.startMs,
				options.endMs,
				options.limit,
			],
			queryFn: () => {
				return api.getTraces(options);
			},
		});
	},
	actorQueueStatusQueryOptions(actorId: ActorId, limit: number) {
		return queryOptions({
			staleTime: 0,
			queryKey: actorInspectorQueriesKeys.actorQueueStatus(
				actorId,
				limit,
			),
			queryFn: () => {
				return api.getQueueStatus(limit);
			},
		});
	},
	actorQueueSizeQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorQueueSize(actorId),
			queryFn: () => 0,
		});
	},

	actorWorkflowHistoryQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorWorkflowHistory(actorId),
			queryFn: () => {
				return api.getWorkflowHistory();
			},
		});
	},

	actorIsWorkflowEnabledQueryOptions(actorId: ActorId) {
		return queryOptions({
			staleTime: Infinity,
			queryKey: actorInspectorQueriesKeys.actorIsWorkflowEnabled(actorId),
			queryFn: () => false,
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
> & {
	connectionStatus: ConnectionStatus;
	isInspectorAvailable: boolean;
	rivetkitVersion?: string;
	inspectorProtocolVersion: number;
	features: {
		traces: FeatureSupport;
		queue: FeatureSupport;
	};
};

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

	const { data: actorMetadata, isSuccess: isActorMetadataSuccess } = useQuery(
		{
			...actorMetadataQueryOptions({ actorId, credentials }),
		},
	);

	const { isSuccess: isActorDataSuccess } = useActorInspectorData(actorId);

	const isInspectorAvailable = isActorMetadataSuccess && isActorDataSuccess;
	const rivetkitVersion = actorMetadata?.version;
	const inspectorProtocolVersion = useMemo(
		() => getInspectorProtocolVersion(rivetkitVersion),
		[rivetkitVersion],
	);
	const features = useMemo(
		() => ({
			traces: buildFeatureSupport(
				rivetkitVersion,
				MIN_RIVETKIT_VERSION_TRACES,
				"Traces",
			),
			queue: buildFeatureSupport(
				rivetkitVersion,
				MIN_RIVETKIT_VERSION_QUEUE,
				"Queue",
			),
		}),
		[rivetkitVersion],
	);

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
					actionsManager.current.createResolver<unknown>({
						name: "executeAction",
					});

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "ActionRequest",
								val: {
									id: BigInt(id),
									name,
									args: new Uint8Array(cbor.encode(args))
										.buffer,
								},
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			patchState: async (state) => {
				sendMessage(
					serverMessage(
						{
							body: {
								tag: "PatchStateRequest",
								val: {
									state: new Uint8Array(cbor.encode(state))
										.buffer,
								},
							},
						},
						inspectorProtocolVersion,
					),
				);
			},

			getConnections: async () => {
				const { id, promise } = actionsManager.current.createResolver<
					Connection[]
				>({ name: "getConnections" });

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "ConnectionsRequest",
								val: { id: BigInt(id) },
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			getState: async () => {
				const { id, promise } = actionsManager.current.createResolver<{
					isEnabled: boolean;
					state: unknown;
				}>({ name: "getState" });

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "StateRequest",
								val: { id: BigInt(id) },
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			getRpcs() {
				const { id, promise } = actionsManager.current.createResolver<
					string[]
				>({ name: "getRpcs" });

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "RpcsListRequest",
								val: { id: BigInt(id) },
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},
			getTraces: async ({ startMs, endMs, limit }) => {
				const { id, promise } =
					actionsManager.current.createResolver<ReadRangeWire>({
						name: "getTraces",
						timeoutMs: 10_000,
					});

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "TraceQueryRequest",
								val: {
									id: BigInt(id),
									startMs: BigInt(Math.floor(startMs)),
									endMs: BigInt(Math.floor(endMs)),
									limit: BigInt(limit),
								},
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},
			getQueueStatus: async (limit) => {
				const safeLimit = Math.max(0, Math.floor(limit));
				const { id, promise } =
					actionsManager.current.createResolver<QueueStatusSummary>({
						name: "getQueueStatus",
						timeoutMs: 10_000,
					});

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "QueueRequest",
								val: {
									id: BigInt(id),
									limit: BigInt(safeLimit),
								},
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			getWorkflowHistory: async () => {
				const { id, promise } = actionsManager.current.createResolver<{
					history: WorkflowHistory | null;
					isEnabled: boolean;
				}>({
					name: "getWorkflowHistory",
					timeoutMs: 10_000,
				});

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "WorkflowHistoryRequest",
								val: { id: BigInt(id) },
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			getDatabaseSchema: async () => {
				const { id, promise } =
					actionsManager.current.createResolver<DatabaseSchema>({
						name: "getDatabaseSchema",
						timeoutMs: 10_000,
					});

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "DatabaseSchemaRequest",
								val: { id: BigInt(id) },
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			getDatabaseTableRows: async (table, limit, offset) => {
				const { id, promise } = actionsManager.current.createResolver<
					unknown[]
				>({
					name: "getDatabaseTableRows",
					timeoutMs: 10_000,
				});

				sendMessage(
					serverMessage(
						{
							body: {
								tag: "DatabaseTableRowsRequest",
								val: {
									id: BigInt(id),
									table,
									limit: BigInt(limit),
									offset: BigInt(offset),
								},
							},
						},
						inspectorProtocolVersion,
					),
				);

				return promise;
			},

			getMetadata() {
				return getActorMetadataProxy.current();
			},
		} satisfies ActorInspectorApi;
	}, [sendMessage, reconnect, inspectorProtocolVersion]);

	const value = useMemo(() => {
		return {
			connectionStatus: status,
			isInspectorAvailable,
			rivetkitVersion,
			inspectorProtocolVersion,
			features,
			...createDefaultActorInspectorContext({
				api,
			}),
		};
	}, [
		api,
		status,
		isInspectorAvailable,
		rivetkitVersion,
		inspectorProtocolVersion,
		features,
	]);

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
		let message: ReturnType<typeof toClient.deserializeWithEmbeddedVersion>;
		try {
			message = toClient.deserializeWithEmbeddedVersion(
				new Uint8Array(await e.data.arrayBuffer()),
			);
		} catch (error) {
			console.warn("Failed to decode inspector message", error);
			return;
		}

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
					actorInspectorQueriesKeys.actorIsStateEnabled(actorId),
					body.val.isStateEnabled,
				);

				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorIsWorkflowEnabled(actorId),
					body.val.isWorkflowEnabled,
				);

				queryClient.setQueryData(
					[
						...actorInspectorQueriesKeys.actorDatabase(actorId),
						"enabled",
					],
					body.val.isDatabaseEnabled,
				);

				if (body.val.workflowHistory) {
					queryClient.setQueryData(
						actorInspectorQueriesKeys.actorWorkflowHistory(actorId),
						transformWorkflowHistoryFromInspector(
							body.val.workflowHistory,
						),
					);
				}
			})
			.with({ tag: "ConnectionsResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					transformConnections(body.val.connections),
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
			.with({ tag: "RpcsListResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(Number(rid), body.val.rpcs);
			})
			.with({ tag: "TraceQueryResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					decodeReadRangeWire(new Uint8Array(body.val.payload)),
				);
			})
			.with({ tag: "QueueResponse" }, (body) => {
				const { rid, status } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					normalizeQueueStatus(status),
				);
			})
			.with({ tag: "QueueUpdated" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorQueueSize(actorId),
					Number(body.val.queueSize),
				);
				queryClient.invalidateQueries({
					queryKey: ["actor", actorId, "queue"],
				});
			})
			.with({ tag: "WorkflowHistoryUpdated" }, (body) => {
				queryClient.setQueryData(
					actorInspectorQueriesKeys.actorWorkflowHistory(actorId),
					transformWorkflowHistoryFromInspector(body.val.history),
				);
			})
			.with({ tag: "WorkflowHistoryResponse" }, (body) => {
				const { rid } = body.val;
				const transformed = body.val.history
					? transformWorkflowHistoryFromInspector(body.val.history)
					: null;
				actionsManager.current.resolve(Number(rid), {
					history: transformed?.history ?? null,
					isEnabled: body.val.isWorkflowEnabled,
				});
			})
			.with({ tag: "DatabaseSchemaResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					cbor.decode(new Uint8Array(body.val.schema)),
				);
			})
			.with({ tag: "DatabaseTableRowsResponse" }, (body) => {
				const { rid } = body.val;
				actionsManager.current.resolve(
					Number(rid),
					cbor.decode(new Uint8Array(body.val.result)),
				);
			})
			.with({ tag: "Error" }, (body) => {
				if (body.val.message === INSPECTOR_ERROR_EVENTS_DROPPED) {
					return;
				}
				toast.error(`Inspector error: ${body.val.message}`);
			})
			.exhaustive();
	};

function transformConnections(connections: readonly Connection[]) {
	return connections.map((connection) => ({
		...connection,
		details: cbor.decode(new Uint8Array(connection.details)),
	}));
}

function transformState(state: ArrayBuffer) {
	return cbor.decode(new Uint8Array(state));
}

function transformWorkflowHistoryFromInspector(raw: ArrayBuffer): {
	history: WorkflowHistory | null;
	isEnabled: boolean;
} {
	try {
		const decoded = decodeWorkflowHistoryTransport(raw);
		return {
			history: transformWorkflowHistory(decoded),
			isEnabled: true,
		};
	} catch (error) {
		console.warn("Failed to decode workflow history", error);
		return { history: null, isEnabled: true };
	}
}

function serverMessage(data: ToServer, version: number) {
	return toServer.serializeWithEmbeddedVersion(data, version);
}

class ActionsManager {
	private suspensions = new Map<number, PromiseWithResolvers<any>>();

	private nextId = 1;

	createResolver<T = void>(options?: {
		timeoutMs?: number;
		name?: string;
	}): { id: number; promise: Promise<T> } {
		const id = this.nextId++;
		const { promise, resolve, reject } = Promise.withResolvers<T>();
		this.suspensions.set(id, { promise, resolve, reject });
		const timeoutMs = options?.timeoutMs ?? 2_000;

		// set a timeout to reject the promise if not resolved in time
		setTimeout(() => {
			if (this.suspensions.has(id)) {
				reject(
					new Error(
						`Action timed out: ${options?.name ?? "unknown"}`,
					),
				);
				this.suspensions.delete(id);
			}
		}, timeoutMs);

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
