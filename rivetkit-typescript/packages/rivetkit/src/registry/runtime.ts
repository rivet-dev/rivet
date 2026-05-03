import type { SqliteNativeMetrics } from "@/common/database/config";
import type { RegistryConfig } from "./config";

declare const handleBrand: unique symbol;

type OpaqueHandle<Name extends string> = {
	readonly [handleBrand]: Name;
};

export type RegistryHandle = OpaqueHandle<"registry">;
export type ActorFactoryHandle = OpaqueHandle<"actorFactory">;
export type ActorContextHandle = OpaqueHandle<"actorContext">;
export type ConnHandle = OpaqueHandle<"conn">;
export type WebSocketHandle = OpaqueHandle<"webSocket">;
export type CancellationTokenHandle = OpaqueHandle<"cancellationToken">;

export type RuntimeBytes = Uint8Array;

export interface RuntimeActorKeySegment {
	kind: string;
	stringValue?: string;
	numberValue?: number;
}

export interface RuntimeHttpRequest {
	method: string;
	uri: string;
	headers?: Record<string, string>;
	body?: RuntimeBytes;
}

export interface RuntimeHttpResponse {
	status?: number;
	headers?: Record<string, string>;
	body?: RuntimeBytes;
}

export interface RuntimeStateDeltaPayload {
	state?: RuntimeBytes;
	connHibernation: Array<{
		connId: string;
		bytes: RuntimeBytes;
	}>;
	connHibernationRemoved: string[];
}

export interface RuntimeRequestSaveOpts {
	immediate?: boolean;
	maxWaitMs?: number;
}

export interface RuntimeInspectorSnapshot {
	stateRevision: number;
	connectionsRevision: number;
	queueRevision: number;
	activeConnections: number;
	queueSize: number;
	connectedClients: number;
}

export interface RuntimeQueueMessage {
	id(): bigint;
	name(): string;
	body(): RuntimeBytes;
	createdAt(): number;
	isCompletable(): boolean;
	complete(response?: RuntimeBytes | undefined | null): Promise<void>;
}

export interface RuntimeQueueInspectMessage {
	id: number;
	name: string;
	createdAtMs: number;
}

export interface RuntimeQueueSendResult {
	status: string;
	response?: RuntimeBytes;
}

export interface RuntimeQueueNextBatchOptions {
	names?: string[];
	count?: number;
	timeoutMs?: number;
	completable?: boolean;
}

export interface RuntimeQueueWaitOptions {
	timeoutMs?: number;
	completable?: boolean;
}

export interface RuntimeQueueEnqueueAndWaitOptions {
	timeoutMs?: number;
}

export interface RuntimeQueueTryNextBatchOptions {
	names?: string[];
	count?: number;
	completable?: boolean;
}

export interface RuntimeKvListOptions {
	reverse?: boolean;
	limit?: number;
}

export interface RuntimeKvEntry {
	key: RuntimeBytes;
	value: RuntimeBytes;
}

type RuntimeSqlBindNoValues = {
	intValue?: never;
	floatValue?: never;
	textValue?: never;
	blobValue?: never;
};

export type RuntimeSqlBindParam =
	| ({ kind: "null" } & RuntimeSqlBindNoValues)
	| {
			kind: "int";
			intValue: number;
			floatValue?: never;
			textValue?: never;
			blobValue?: never;
	  }
	| {
			kind: "float";
			intValue?: never;
			floatValue: number;
			textValue?: never;
			blobValue?: never;
	  }
	| {
			kind: "text";
			intValue?: never;
			floatValue?: never;
			textValue: string;
			blobValue?: never;
	  }
	| {
			kind: "blob";
			intValue?: never;
			floatValue?: never;
			textValue?: never;
			blobValue: RuntimeBytes;
	  };

export type RuntimeSqlBindParams = RuntimeSqlBindParam[] | null;

export interface RuntimeSqlQueryResult {
	columns: string[];
	rows: unknown[][];
}

export type RuntimeSqlExecResult = RuntimeSqlQueryResult;

export interface RuntimeSqlExecuteResult extends RuntimeSqlQueryResult {
	changes: number;
	lastInsertRowId?: number | null;
}

export function normalizeRuntimeSqlExecuteResult(
	result: RuntimeSqlQueryResult & {
		changes: number;
		lastInsertRowId?: number | null;
	},
): RuntimeSqlExecuteResult {
	return result;
}

export interface RuntimeSqlRunResult {
	changes: number;
}

export interface RuntimeSqlDatabase {
	exec(sql: string): Promise<RuntimeSqlExecResult>;
	execute(
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlExecuteResult>;
	query(
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlQueryResult>;
	run(
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlRunResult>;
	metrics?(): SqliteNativeMetrics | null;
	takeLastKvError?(): string | null;
	close(): Promise<void>;
}

export interface RuntimeActorConfig {
	name?: string;
	icon?: string;
	hasDatabase?: boolean;
	remoteSqlite?: boolean;
	hasState?: boolean;
	canHibernateWebsocket?: boolean;
	stateSaveIntervalMs?: number;
	createStateTimeoutMs?: number;
	onCreateTimeoutMs?: number;
	createVarsTimeoutMs?: number;
	createConnStateTimeoutMs?: number;
	onBeforeConnectTimeoutMs?: number;
	onConnectTimeoutMs?: number;
	onMigrateTimeoutMs?: number;
	onWakeTimeoutMs?: number;
	onBeforeActorStartTimeoutMs?: number;
	actionTimeoutMs?: number;
	onRequestTimeoutMs?: number;
	sleepTimeoutMs?: number;
	noSleep?: boolean;
	sleepGracePeriodMs?: number;
	connectionLivenessTimeoutMs?: number;
	connectionLivenessIntervalMs?: number;
	maxQueueSize?: number;
	maxQueueMessageSize?: number;
	maxIncomingMessageSize?: number;
	maxOutgoingMessageSize?: number;
	preloadMaxWorkflowBytes?: number;
	preloadMaxConnectionsBytes?: number;
	actions?: Array<{ name: string }>;
}

export interface RuntimeServeConfig {
	version: number;
	endpoint: string;
	token?: string;
	namespace: string;
	poolName: string;
	engineBinaryPath?: string;
	handleInspectorHttpInRuntime?: boolean;
	inspectorTestToken?: string;
	serverlessBasePath?: string;
	serverlessPackageVersion: string;
	serverlessClientEndpoint?: string;
	serverlessClientNamespace?: string;
	serverlessClientToken?: string;
	serverlessValidateEndpoint: boolean;
	serverlessMaxStartPayloadBytes: number;
}

export interface RuntimeServerlessRequest {
	method: string;
	url: string;
	headers: Record<string, string>;
	body: RuntimeBytes;
}

export interface RuntimeServerlessResponseHead {
	status: number;
	headers: Record<string, string>;
}

export interface RuntimeRegistryDiagnostics {
	mode: string;
	envoyActiveActorCount?: number | null;
}

export type RuntimeServerlessStreamEvent =
	| {
			kind: "chunk";
			chunk?: RuntimeBytes;
	  }
	| {
			kind: "end";
			error?: {
				group: string;
				code: string;
				message: string;
			};
	  };

export type RuntimeServerlessStreamCallback = (
	error: unknown,
	event?: RuntimeServerlessStreamEvent,
) => unknown;

export type RuntimeWebSocketEvent =
	| {
			kind: "message";
			data: string | RuntimeBytes;
			binary: boolean;
			messageIndex?: number;
	  }
	| {
			kind: "close";
			code: number;
			reason: string;
			wasClean: boolean;
	  };

export interface CoreRuntime {
	readonly kind: "napi" | "wasm";

	createRegistry(): RegistryHandle;
	registerActor(
		registry: RegistryHandle,
		name: string,
		factory: ActorFactoryHandle,
	): void;
	serveRegistry(
		registry: RegistryHandle,
		config: RuntimeServeConfig,
	): Promise<void>;
	shutdownRegistry(registry: RegistryHandle): Promise<void>;
	handleServerlessRequest(
		registry: RegistryHandle,
		req: RuntimeServerlessRequest,
		onStreamEvent: RuntimeServerlessStreamCallback,
		cancelToken: CancellationTokenHandle,
		config: RuntimeServeConfig,
	): Promise<RuntimeServerlessResponseHead>;
	registryDiagnostics?(
		registry: RegistryHandle,
	): Promise<RuntimeRegistryDiagnostics>;
	createActorFactory(
		callbacks: object,
		config?: RuntimeActorConfig | undefined | null,
	): ActorFactoryHandle;

	createCancellationToken(): CancellationTokenHandle;
	cancellationTokenAborted(token: CancellationTokenHandle): boolean;
	cancelCancellationToken(token: CancellationTokenHandle): void;
	onCancellationTokenCancelled(
		token: CancellationTokenHandle,
		callback: (...args: unknown[]) => unknown,
	): void;

	actorState(ctx: ActorContextHandle): RuntimeBytes;
	actorBeginOnStateChange(ctx: ActorContextHandle): void;
	actorEndOnStateChange(ctx: ActorContextHandle): void;
	actorSetAlarm(
		ctx: ActorContextHandle,
		timestampMs?: number | undefined | null,
	): void;
	actorRequestSave(
		ctx: ActorContextHandle,
		opts?: RuntimeRequestSaveOpts | undefined | null,
	): void;
	actorRequestSaveAndWait(
		ctx: ActorContextHandle,
		opts?: RuntimeRequestSaveOpts | undefined | null,
	): Promise<void>;
	actorInspectorSnapshot(ctx: ActorContextHandle): RuntimeInspectorSnapshot;
	actorDecodeInspectorRequest(
		ctx: ActorContextHandle,
		bytes: RuntimeBytes,
		advertisedVersion: number,
	): RuntimeBytes;
	actorEncodeInspectorResponse(
		ctx: ActorContextHandle,
		bytes: RuntimeBytes,
		targetVersion: number,
	): RuntimeBytes;
	actorVerifyInspectorAuth(
		ctx: ActorContextHandle,
		bearerToken?: string | undefined | null,
	): Promise<void>;
	actorQueueHibernationRemoval(ctx: ActorContextHandle, connId: string): void;
	actorTakePendingHibernationChanges(ctx: ActorContextHandle): string[];
	actorDirtyHibernatableConns(ctx: ActorContextHandle): ConnHandle[];
	actorSaveState(
		ctx: ActorContextHandle,
		payload: RuntimeStateDeltaPayload,
	): Promise<void>;
	actorId(ctx: ActorContextHandle): string;
	actorName(ctx: ActorContextHandle): string;
	actorKey(ctx: ActorContextHandle): RuntimeActorKeySegment[];
	actorRegion(ctx: ActorContextHandle): string;
	actorSleep(ctx: ActorContextHandle): void;
	actorDestroy(ctx: ActorContextHandle): void;
	actorAbortSignal(ctx: ActorContextHandle): AbortSignal;
	actorConns(ctx: ActorContextHandle): ConnHandle[];
	actorConnectConn(
		ctx: ActorContextHandle,
		params: RuntimeBytes,
		request?: RuntimeHttpRequest | undefined | null,
	): Promise<ConnHandle>;
	actorBroadcast(
		ctx: ActorContextHandle,
		name: string,
		args: RuntimeBytes,
	): void;
	actorWaitUntil(ctx: ActorContextHandle, promise: Promise<unknown>): void;
	actorKeepAwake(
		ctx: ActorContextHandle,
		promise: Promise<unknown>,
	): Promise<unknown>;
	actorRegisterTask(ctx: ActorContextHandle, promise: Promise<unknown>): void;
	actorRuntimeState(ctx: ActorContextHandle): object;
	actorClearRuntimeState(ctx: ActorContextHandle): void;
	actorRestartRunHandler(ctx: ActorContextHandle): void;
	actorBeginWebsocketCallback(ctx: ActorContextHandle): number;
	actorEndWebsocketCallback(ctx: ActorContextHandle, regionId: number): void;

	actorKvGet(
		ctx: ActorContextHandle,
		key: RuntimeBytes,
	): Promise<RuntimeBytes | null>;
	actorKvPut(
		ctx: ActorContextHandle,
		key: RuntimeBytes,
		value: RuntimeBytes,
	): Promise<void>;
	actorKvDelete(ctx: ActorContextHandle, key: RuntimeBytes): Promise<void>;
	actorKvDeleteRange(
		ctx: ActorContextHandle,
		start: RuntimeBytes,
		end: RuntimeBytes,
	): Promise<void>;
	actorKvListPrefix(
		ctx: ActorContextHandle,
		prefix: RuntimeBytes,
		options?: RuntimeKvListOptions | undefined | null,
	): Promise<RuntimeKvEntry[]>;
	actorKvListRange(
		ctx: ActorContextHandle,
		start: RuntimeBytes,
		end: RuntimeBytes,
		options?: RuntimeKvListOptions | undefined | null,
	): Promise<RuntimeKvEntry[]>;
	actorKvBatchGet(
		ctx: ActorContextHandle,
		keys: RuntimeBytes[],
	): Promise<Array<RuntimeBytes | undefined | null>>;
	actorKvBatchPut(
		ctx: ActorContextHandle,
		entries: RuntimeKvEntry[],
	): Promise<void>;
	actorKvBatchDelete(
		ctx: ActorContextHandle,
		keys: RuntimeBytes[],
	): Promise<void>;

	actorSqlExec(
		ctx: ActorContextHandle,
		sql: string,
	): Promise<RuntimeSqlExecResult>;
	actorSqlExecute(
		ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlExecuteResult>;
	actorSqlQuery(
		ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlQueryResult>;
	actorSqlRun(
		ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlRunResult>;
	actorSqlMetrics(ctx: ActorContextHandle): SqliteNativeMetrics | null;
	actorSqlTakeLastKvError(ctx: ActorContextHandle): string | null;
	actorSqlClose(ctx: ActorContextHandle): Promise<void>;

	actorQueueSend(
		ctx: ActorContextHandle,
		name: string,
		body: RuntimeBytes,
	): Promise<RuntimeQueueMessage>;
	actorQueueNextBatch(
		ctx: ActorContextHandle,
		options?: RuntimeQueueNextBatchOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeQueueMessage[]>;
	actorQueueWaitForNames(
		ctx: ActorContextHandle,
		names: string[],
		options?: RuntimeQueueWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeQueueMessage>;
	actorQueueWaitForNamesAvailable(
		ctx: ActorContextHandle,
		names: string[],
		options?: RuntimeQueueWaitOptions | undefined | null,
	): Promise<void>;
	actorQueueEnqueueAndWait(
		ctx: ActorContextHandle,
		name: string,
		body: RuntimeBytes,
		options?: RuntimeQueueEnqueueAndWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeBytes | null>;
	actorQueueTryNextBatch(
		ctx: ActorContextHandle,
		options?: RuntimeQueueTryNextBatchOptions | undefined | null,
	): RuntimeQueueMessage[];
	actorQueueMaxSize(ctx: ActorContextHandle): number;
	actorQueueInspectMessages(
		ctx: ActorContextHandle,
	): Promise<RuntimeQueueInspectMessage[]>;

	actorScheduleAfter(
		ctx: ActorContextHandle,
		durationMs: number,
		actionName: string,
		args: RuntimeBytes,
	): void;
	actorScheduleAt(
		ctx: ActorContextHandle,
		timestampMs: number,
		actionName: string,
		args: RuntimeBytes,
	): void;

	connId(conn: ConnHandle): string;
	connParams(conn: ConnHandle): RuntimeBytes;
	connState(conn: ConnHandle): RuntimeBytes;
	connSetState(conn: ConnHandle, state: RuntimeBytes): void;
	connIsHibernatable(conn: ConnHandle): boolean;
	connSend(conn: ConnHandle, name: string, args: RuntimeBytes): void;
	connDisconnect(
		conn: ConnHandle,
		reason?: string | undefined | null,
	): Promise<void>;

	webSocketSend(
		ws: WebSocketHandle,
		data: RuntimeBytes,
		binary: boolean,
	): void;
	webSocketClose(
		ws: WebSocketHandle,
		code?: number | undefined | null,
		reason?: string | undefined | null,
	): Promise<void>;
	webSocketSetEventCallback(
		ws: WebSocketHandle,
		callback: (event: RuntimeWebSocketEvent) => void,
	): void;
}

export interface RuntimeBundle {
	runtime: CoreRuntime;
}

export async function buildServeConfig(
	config: RegistryConfig,
	loadEnginePath: () => Promise<string>,
	version: string,
): Promise<RuntimeServeConfig> {
	if (!config.endpoint) {
		throw new Error("registry endpoint is required");
	}

	const serveConfig: RuntimeServeConfig = {
		version: config.envoy.version,
		endpoint: config.endpoint,
		token: config.token,
		namespace: config.namespace,
		poolName: config.envoy.poolName,
		handleInspectorHttpInRuntime: true,
		serverlessBasePath: config.serverless.basePath,
		serverlessPackageVersion: version,
		serverlessClientEndpoint: config.publicEndpoint,
		serverlessClientNamespace: config.publicNamespace,
		serverlessClientToken: config.publicToken,
		serverlessValidateEndpoint: config.validateServerlessEndpoint,
		serverlessMaxStartPayloadBytes: config.serverless.maxStartPayloadBytes,
	};

	if (config.startEngine) {
		serveConfig.engineBinaryPath = await loadEnginePath();
	}
	if (config.test?.enabled) {
		serveConfig.inspectorTestToken =
			process.env._RIVET_TEST_INSPECTOR_TOKEN ?? "token";
	}

	return serveConfig;
}
