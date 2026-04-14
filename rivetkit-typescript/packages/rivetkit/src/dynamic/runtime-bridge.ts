export type ActorKey = string[];
export type Encoding = "json" | "cbor" | "bare";
export type DynamicSourceFormat = "commonjs-js" | "esm-js";
/**
 * Canonical binary transport type for host<->isolate envelopes.
 *
 * API surfaces may use `Buffer` or `Uint8Array`, but boundary messages should
 * normalize binary payloads to `ArrayBuffer`.
 */
export type BridgeBinary = ArrayBuffer;

/**
 * Isolate global key where the host injects actor identity/config before
 * loading the dynamic bootstrap module.
 */
export const DYNAMIC_BOOTSTRAP_CONFIG_GLOBAL_KEY =
	"__rivetkitDynamicBootstrapConfig";

/**
 * Host -> isolate bridge keys.
 *
 * Each key points to an `isolated-vm` reference injected by the host runtime
 * so isolate code can call back into host services (KV, alarms, client calls,
 * websocket dispatch, and logging).
 */
export const DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS = {
	kvBatchPut: "__rivetkitDynamicHostKvBatchPut",
	kvBatchGet: "__rivetkitDynamicHostKvBatchGet",
	kvBatchDelete: "__rivetkitDynamicHostKvBatchDelete",
	kvDeleteRange: "__rivetkitDynamicHostKvDeleteRange",
	kvListPrefix: "__rivetkitDynamicHostKvListPrefix",
	kvListRange: "__rivetkitDynamicHostKvListRange",
	dbExec: "__rivetkitDynamicHostDbExec",
	dbQuery: "__rivetkitDynamicHostDbQuery",
	dbRun: "__rivetkitDynamicHostDbRun",
	dbClose: "__rivetkitDynamicHostDbClose",
	setAlarm: "__rivetkitDynamicHostSetAlarm",
	clientCall: "__rivetkitDynamicHostClientCall",
	rawDatabaseExecute: "__rivetkitDynamicHostRawDatabaseExecute",
	ackHibernatableWebSocketMessage:
		"__rivetkitDynamicHostAckHibernatableWebSocketMessage",
	startSleep: "__rivetkitDynamicHostStartSleep",
	startDestroy: "__rivetkitDynamicHostStartDestroy",
	dispatch: "__rivetkitDynamicHostDispatch",
	log: "__rivetkitDynamicHostLog",
} as const;

/**
 * Isolate export -> global keys.
 *
 * After requiring the bootstrap module, the host copies each exported envelope
 * handler onto these globals, then captures references for fast invocation.
 */
export const DYNAMIC_ISOLATE_EXPORT_GLOBAL_KEYS = {
	dynamicFetchEnvelope: "__rivetkitDynamicFetchEnvelope",
	dynamicDispatchAlarmEnvelope: "__rivetkitDynamicDispatchAlarmEnvelope",
	dynamicStopEnvelope: "__rivetkitDynamicStopEnvelope",
	dynamicOpenWebSocketEnvelope: "__rivetkitDynamicOpenWebSocketEnvelope",
	dynamicWebSocketSendEnvelope: "__rivetkitDynamicWebSocketSendEnvelope",
	dynamicWebSocketCloseEnvelope: "__rivetkitDynamicWebSocketCloseEnvelope",
	dynamicGetHibernatingWebSocketsEnvelope:
		"__rivetkitDynamicGetHibernatingWebSocketsEnvelope",
	dynamicCleanupPersistedConnectionsEnvelope:
		"__rivetkitDynamicCleanupPersistedConnectionsEnvelope",
	dynamicEnsureStartedEnvelope: "__rivetkitDynamicEnsureStartedEnvelope",
	dynamicDisposeEnvelope: "__rivetkitDynamicDisposeEnvelope",
} as const;

export type DynamicBootstrapExportName =
	keyof typeof DYNAMIC_ISOLATE_EXPORT_GLOBAL_KEYS;

export interface DynamicBootstrapConfig {
	/** Concrete actor id for the isolate instance. */
	actorId: string;
	/** Actor definition name used to build a one-actor registry in isolate. */
	actorName: string;
	/** Actor key used for actor startup and request routing. */
	actorKey: ActorKey;
	/** Engine endpoint for native SQLite fallback inside the isolate. */
	endpoint: string;
	/** Namespace for native SQLite fallback inside the isolate. */
	namespace: string;
	/** Auth token for native SQLite fallback inside the isolate. */
	token?: string;
	/** Runtime source module file name written under the actor runtime dir. */
	sourceEntry: string;
	/** Module format for the runtime source file entrypoint. */
	sourceFormat: DynamicSourceFormat;
}

/** Serialized HTTP request envelope crossing host<->isolate boundary. */
export interface FetchEnvelopeInput {
	url: string;
	method: string;
	headers: Record<string, string>;
	bodyBase64?: string;
}

/** Serialized HTTP response envelope crossing host<->isolate boundary. */
export interface FetchEnvelopeOutput {
	status: number;
	headers: Array<[string, string]>;
	body: BridgeBinary;
}

/** Host instruction to open an actor websocket inside isolate. */
export interface WebSocketOpenEnvelopeInput {
	sessionId: number;
	path: string;
	encoding: Encoding;
	params: unknown;
	headers?: Record<string, string>;
	gatewayId?: BridgeBinary;
	requestId?: BridgeBinary;
	isHibernatable?: boolean;
	isRestoringHibernatable?: boolean;
}

/** Host instruction to forward websocket message data into isolate. */
export interface WebSocketSendEnvelopeInput {
	sessionId: number;
	kind: "text" | "binary";
	text?: string;
	data?: BridgeBinary;
	rivetMessageIndex?: number;
}

/** Host instruction to close an isolate websocket session. */
export interface WebSocketCloseEnvelopeInput {
	sessionId: number;
	code?: number;
	reason?: string;
}

/** Serialized dynamic inline client call from isolate back to host. */
export interface DynamicClientCallInput {
	actorName: string;
	accessorMethod: "get" | "getOrCreate" | "getForId" | "create";
	accessorArgs: unknown[];
	operation: string;
	operationArgs: unknown[];
}

/** Serialized websocket event payload emitted by isolate back to host. */
export type IsolateDispatchPayload =
	| {
			type: "open";
			sessionId: number;
	  }
	| {
			type: "message";
			sessionId: number;
			kind: "text" | "binary";
			text?: string;
			data?: BridgeBinary;
			rivetMessageIndex?: number;
	  }
	| {
			type: "close";
			sessionId: number;
			code?: number;
			reason?: string;
			wasClean?: boolean;
	  }
	| {
			type: "error";
			sessionId: number;
			message?: string;
	  };

export interface DynamicHibernatingWebSocketMetadata {
	/** Gateway id associated with the hibernatable websocket. */
	gatewayId: ArrayBuffer;
	/** Request id associated with the hibernatable websocket. */
	requestId: ArrayBuffer;
	/** Last persisted server message index. */
	serverMessageIndex: number;
	/** Last seen client message index. */
	clientMessageIndex: number;
	/** Original websocket request path. */
	path: string;
	/** Original websocket request headers. */
	headers: Record<string, string>;
}

/**
 * Public shape exported by the dynamic bootstrap module.
 *
 * The host runtime expects every function below to exist and wires each one
 * into the isolate bridge by key.
 */
export interface DynamicBootstrapExports {
	dynamicFetchEnvelope: (
		url: string,
		method: string,
		headers: Record<string, string>,
		bodyBase64?: string | null,
	) => Promise<FetchEnvelopeOutput>;
	dynamicDispatchAlarmEnvelope: () => Promise<boolean>;
	dynamicStopEnvelope: (mode: "sleep" | "destroy") => Promise<boolean>;
	dynamicOpenWebSocketEnvelope: (
		input: WebSocketOpenEnvelopeInput,
	) => Promise<boolean>;
	dynamicWebSocketSendEnvelope: (
		input: WebSocketSendEnvelopeInput,
	) => Promise<boolean>;
	dynamicWebSocketCloseEnvelope: (
		input: WebSocketCloseEnvelopeInput,
	) => Promise<boolean>;
	dynamicGetHibernatingWebSocketsEnvelope: () => Promise<
		Array<DynamicHibernatingWebSocketMetadata>
	>;
	dynamicDisposeEnvelope: () => Promise<boolean>;
}
