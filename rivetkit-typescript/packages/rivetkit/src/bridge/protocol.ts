import type {
	RuntimeActorKeySegment,
	RuntimeBytes,
	RuntimeKvListOptions,
	RuntimeQueueEnqueueAndWaitOptions,
	RuntimeQueueNextBatchOptions,
	RuntimeQueueTryNextBatchOptions,
	RuntimeQueueWaitOptions,
	RuntimeRequestSaveOpts,
	RuntimeSqlBindParams,
	RuntimeStateDeltaPayload,
	RuntimeWebSocketEvent,
} from "@/registry/runtime";

/**
 * Wire protocol between a bridged actor host and its child runtime.
 *
 * The host process owns the real CoreRuntime (NAPI) handles. The child runs
 * the regular `buildNativeFactory` glue against a `RemoteCoreRuntime` and
 * exchanges these envelopes over a `node:worker_threads` MessagePort.
 *
 * All payloads must survive structured clone. Opaque runtime handles are
 * replaced by `BridgeHandleRef` values; binary payloads stay `Uint8Array`.
 */

// MARK: Handle refs

export interface BridgeConnMeta {
	connId: string;
	params: RuntimeBytes;
	isHibernatable: boolean;
	/** Current persisted conn state bytes at the time the conn first crossed the bridge. */
	state: RuntimeBytes;
}

export type BridgeHandleRef =
	/** The singleton actor context for this bridge. */
	| { __bridge: "ctx" }
	/**
	 * Connection handle. Conns are identified by their stable conn id; `meta`
	 * is attached the first time a conn crosses to the child so synchronous
	 * reads (id, params, state, isHibernatable) never need a host round trip.
	 */
	| { __bridge: "conn"; connId: string; meta?: BridgeConnMeta }
	/** Raw websocket handle, allocated per onWebSocket dispatch. */
	| { __bridge: "ws"; id: number }
	/**
	 * Cancellation token. Positive ids are host-created tokens that crossed in
	 * a callback payload; negative ids are child-created tokens mirrored to the
	 * host on demand.
	 */
	| { __bridge: "token"; id: number; aborted?: boolean };

export function isBridgeHandleRef(value: unknown): value is BridgeHandleRef {
	return (
		typeof value === "object" &&
		value !== null &&
		"__bridge" in value &&
		typeof (value as { __bridge?: unknown }).__bridge === "string"
	);
}

// MARK: Bootstrap

/** How the child resolves the actor definition it serves. */
export type BridgeBootstrap =
	| {
			kind: "module";
			/** Module URL or specifier that exports the registry or definition. */
			module: string;
			/** Optional named export to read instead of scanning exports. */
			exportName?: string;
	  }
	| {
			kind: "source";
			/**
			 * Path of the loader-resolved actor source, written by the host
			 * under content-hash directories so identical sources share files.
			 */
			sourcePath: string;
			/** Worker thread resource limits for this actor instance. */
			workerResourceLimits?: {
				maxOldGenerationSizeMb?: number;
			};
	  };

/**
 * Serializable subset of the resolved RegistryConfig that the child needs to
 * rebuild client wiring and validation limits. Derived fields that the
 * registry config transform computes (public endpoint info) ride along.
 */
export interface BridgeRegistryConfig {
	endpoint?: string;
	token?: string;
	namespace: string;
	poolName: string;
	headers: Record<string, string>;
	maxIncomingMessageSize: number;
	maxOutgoingMessageSize: number;
	testEnabled: boolean;
	publicEndpoint?: string;
	publicNamespace?: string;
	publicToken?: string;
}

export interface BridgeWorkerData {
	bootstrap: BridgeBootstrap;
	registryConfig: BridgeRegistryConfig;
	actorName: string;
	actorId: string;
}

/** Immutable actor metadata pushed to the child at spawn. */
export interface BridgeCtxMeta {
	actorId: string;
	actorName: string;
	actorKey: RuntimeActorKeySegment[];
	actorRegion: string;
	queueMaxSize: number;
}

// MARK: Errors

/** Structured error payload crossing the bridge in both directions. */
export interface BridgeErrorPayload {
	message: string;
	/** Set when the source error carried RivetError-style fields. */
	group?: string;
	code?: string;
	metadata?: unknown;
	public?: boolean;
	statusCode?: number;
	stack?: string;
}

// MARK: Queue messages

/** Flattened RuntimeQueueMessage. Completable messages keep a host handle. */
export interface BridgeQueueMessage {
	id: bigint;
	name: string;
	body: RuntimeBytes;
	createdAt: number;
	/** Host handle id for complete() calls; absent when not completable. */
	completableId?: number;
}

// MARK: Host -> child

export type HostToChildMessage =
	/** Invoke a callbacks-bag callback in the child. */
	| {
			kind: "cb:invoke";
			seq: number;
			callback: string;
			/** Action name for action dispatch; unset for other callbacks. */
			actionName?: string;
			payload: Record<string, unknown>;
	  }
	/** Response to a child rpc:call. */
	| {
			kind: "rpc:result";
			seq: number;
			ok: boolean;
			value?: unknown;
			error?: BridgeErrorPayload;
	  }
	/** Websocket event forwarded from the host runtime. */
	| { kind: "evt:websocket"; wsId: number; event: RuntimeWebSocketEvent }
	/** A host-created cancellation token was cancelled. */
	| { kind: "evt:tokenCancelled"; tokenId: number }
	/** The actor abort signal fired. */
	| { kind: "evt:abort" }
	/** A fire-and-forget post from the child failed on the host. */
	| { kind: "evt:postError"; method: string; error: BridgeErrorPayload };

// MARK: Child -> host

export type ChildToHostMessage =
	/**
	 * The child finished bootstrap and is ready for callback dispatch.
	 * `callbackNames` lists the callbacks the loaded definition registered so
	 * the host can short-circuit absent callbacks without a round trip.
	 */
	| { kind: "ready"; callbackNames: string[] }
	/** Bootstrap failed before the child could serve callbacks. */
	| { kind: "bootstrapError"; error: BridgeErrorPayload }
	/** Response to a host cb:invoke. */
	| {
			kind: "cb:result";
			seq: number;
			ok: boolean;
			value?: unknown;
			error?: BridgeErrorPayload;
	  }
	/** Awaited CoreRuntime method call. */
	| { kind: "rpc:call"; seq: number; method: string; args: unknown[] }
	/**
	 * Fire-and-forget CoreRuntime void method. Errors are reported back via
	 * evt:postError instead of throwing at the call site.
	 */
	| { kind: "rpc:post"; method: string; args: unknown[] }
	/**
	 * Blocking synchronous CoreRuntime method. The child parks on
	 * `Atomics.wait` until the host writes the cbor-encoded result into `sab`.
	 */
	| {
			kind: "rpc:sync";
			method: string;
			args: unknown[];
			sab: SharedArrayBuffer;
	  }
	/**
	 * Promise-region begin/end used to mirror promise-argument runtime APIs
	 * (waitUntil, keepAwake, registerTask) and begin/end region APIs
	 * (beginKeepAwake, beginWebsocketCallback) across the boundary.
	 */
	| { kind: "region:begin"; regionId: number; api: BridgeRegionApi }
	| {
			kind: "region:end";
			regionId: number;
			api: BridgeRegionApi;
			error?: BridgeErrorPayload;
	  };

export type BridgeRegionApi =
	| "waitUntil"
	| "keepAwake"
	| "registerTask"
	| "beginKeepAwake"
	| "beginWebsocketCallback";

// MARK: Sync channel layout

/**
 * SharedArrayBuffer layout for rpc:sync:
 * - Int32 word 0: status (see SYNC_STATUS_*), waited on by the child.
 * - Int32 word 1: payload byte length (result on success, error payload on
 *   error, required capacity on overflow).
 * - Bytes from SYNC_HEADER_BYTES: cbor-encoded payload.
 */
export const SYNC_HEADER_BYTES = 8;
export const SYNC_STATUS_PENDING = 0;
export const SYNC_STATUS_OK = 1;
export const SYNC_STATUS_ERROR = 2;
export const SYNC_STATUS_OVERFLOW = 3;
export const SYNC_DEFAULT_BUFFER_BYTES = 256 * 1024;

// MARK: Method classification

/**
 * CoreRuntime methods the child may call as awaited rpc:call requests. The
 * host resolves handle refs in args and translates handle-bearing results.
 */
export const BRIDGE_ASYNC_METHODS = new Set([
	"actorRequestSaveAndWait",
	"actorSaveState",
	"actorVerifyInspectorAuth",
	"actorConnectConn",
	"actorWaitForTrackedShutdownWork",
	"actorWaitForTrackedShutdownWorkUnbounded",
	"actorKvGet",
	"actorKvPut",
	"actorKvDelete",
	"actorKvDeleteRange",
	"actorKvListPrefix",
	"actorKvListRange",
	"actorKvBatchGet",
	"actorKvBatchPut",
	"actorKvBatchDelete",
	"actorSqlExec",
	"actorSqlExecute",
	"actorSqlQuery",
	"actorSqlRun",
	"actorSqlClose",
	"actorQueueSend",
	"actorQueueNextBatch",
	"actorQueueWaitForNames",
	"actorQueueWaitForNamesAvailable",
	"actorQueueEnqueueAndWait",
	"actorQueueInspectMessages",
	"connDisconnect",
	"webSocketClose",
	"queueMessageComplete",
]);

/**
 * Fire-and-forget void methods. Posted without waiting; failures surface as
 * evt:postError and are logged in the child rather than thrown at the call
 * site. Port FIFO ordering keeps them ordered relative to rpc calls.
 */
export const BRIDGE_POST_METHODS = new Set([
	"actorBeginOnStateChange",
	"actorEndOnStateChange",
	"actorSetAlarm",
	"actorRequestSave",
	"actorQueueHibernationRemoval",
	"actorSleep",
	"actorDestroy",
	"actorBroadcast",
	"actorScheduleAfter",
	"actorScheduleAt",
	"actorClearRuntimeState",
	"actorRestartRunHandler",
	"connSend",
	"connSetState",
	"webSocketSend",
	"tokenCreate",
	"tokenCancel",
]);

/**
 * Blocking sync methods served over the SharedArrayBuffer channel. All are
 * cold paths (once-per-wake reads, inspector, shutdown bookkeeping).
 */
export const BRIDGE_SYNC_METHODS = new Set([
	"actorState",
	"actorConns",
	"actorTakePendingHibernationChanges",
	"actorDirtyHibernatableConns",
	"actorQueueTryNextBatch",
	"actorSqlMetrics",
	"actorSqlTakeLastKvError",
	"actorInspectorSnapshot",
	"actorDecodeInspectorRequest",
	"actorEncodeInspectorResponse",
]);

export type {
	RuntimeBytes,
	RuntimeKvListOptions,
	RuntimeQueueEnqueueAndWaitOptions,
	RuntimeQueueNextBatchOptions,
	RuntimeQueueTryNextBatchOptions,
	RuntimeQueueWaitOptions,
	RuntimeRequestSaveOpts,
	RuntimeSqlBindParams,
	RuntimeStateDeltaPayload,
};
