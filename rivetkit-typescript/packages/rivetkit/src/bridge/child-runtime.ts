import type { MessagePort } from "node:worker_threads";
import { getLogger } from "@/common/log";
import type {
	ActorContextHandle,
	ActorFactoryHandle,
	CancellationTokenHandle,
	ConnHandle,
	CoreRuntime,
	RegistryHandle,
	RuntimeActorConfig,
	RuntimeActorKeySegment,
	RuntimeBytes,
	RuntimeInspectorSnapshot,
	RuntimeKvEntry,
	RuntimeKvListOptions,
	RuntimeQueueEnqueueAndWaitOptions,
	RuntimeQueueInspectMessage,
	RuntimeQueueMessage,
	RuntimeQueueNextBatchOptions,
	RuntimeQueueTryNextBatchOptions,
	RuntimeQueueWaitOptions,
	RuntimeRequestSaveOpts,
	RuntimeSqlBindParams,
	RuntimeSqlExecResult,
	RuntimeSqlExecuteResult,
	RuntimeSqlQueryResult,
	RuntimeSqlRunResult,
	RuntimeStateDeltaPayload,
	RuntimeWebSocketEvent,
	WebSocketHandle,
} from "@/registry/runtime";
import { fromBridgeErrorPayload, toBridgeErrorPayload } from "./errors";
import {
	type BridgeCtxMeta,
	type BridgeErrorPayload,
	type BridgeHandleRef,
	type BridgeQueueMessage,
	type BridgeRegionApi,
	type ChildToHostMessage,
	isBridgeHandleRef,
} from "./protocol";
import { SyncCaller } from "./sync-channel";

function logger() {
	return getLogger("actor-bridge-child");
}

interface Deferred {
	resolve: (value: unknown) => void;
	reject: (error: unknown) => void;
}

/** Child-side opaque handle stubs. The glue never inspects these. */
class CtxStub {
	readonly bridgeKind = "ctx" as const;
}

class ConnStub {
	readonly bridgeKind = "conn" as const;
	constructor(
		public readonly connId: string,
		public params: RuntimeBytes,
		public isHibernatable: boolean,
		public state: RuntimeBytes,
	) {}
}

class WsStub {
	readonly bridgeKind = "ws" as const;
	callback?: (event: RuntimeWebSocketEvent) => void;
	/** Events forwarded before the glue registered its callback. */
	buffered: RuntimeWebSocketEvent[] = [];
	constructor(public readonly id: number) {}
}

class TokenStub {
	readonly bridgeKind = "token" as const;
	aborted = false;
	listeners: Array<(...args: unknown[]) => unknown> = [];
	constructor(public readonly id: number) {}
}

class QueueMessageStub implements RuntimeQueueMessage {
	constructor(
		private readonly message: BridgeQueueMessage,
		private readonly runtime: RemoteCoreRuntime,
	) {}

	id(): bigint {
		return this.message.id;
	}
	name(): string {
		return this.message.name;
	}
	body(): RuntimeBytes {
		return this.message.body;
	}
	createdAt(): number {
		return this.message.createdAt;
	}
	isCompletable(): boolean {
		return this.message.completableId !== undefined;
	}
	async complete(response?: RuntimeBytes | undefined | null): Promise<void> {
		if (this.message.completableId === undefined) {
			throw new Error("queue message is not completable");
		}
		await this.runtime.completeQueueMessage(
			this.message.completableId,
			response ?? undefined,
		);
	}
}

function registryMethodError(method: string): Error {
	return new Error(
		`CoreRuntime.${method} is not available inside a bridged actor child; registry serving stays in the host process`,
	);
}

/**
 * CoreRuntime implementation that forwards calls to the host process over a
 * worker_threads MessagePort. One instance serves exactly one actor.
 *
 * Method transport classification (async rpc, fire-and-forget post, blocking
 * sync, or child-local) follows the tables in `protocol.ts`. Immutable handle
 * metadata is pushed eagerly by the host so synchronous reads (actor identity,
 * conn id/params/state) never need a host round trip.
 */
export class RemoteCoreRuntime implements CoreRuntime {
	readonly kind = "napi" as const;

	#port: MessagePort;
	#ctxMeta: BridgeCtxMeta;
	#ctxStub = new CtxStub();
	#nextSeq = 1;
	#pending = new Map<number, Deferred>();
	#syncCaller = new SyncCaller();

	#conns = new Map<string, ConnStub>();
	#wsStubs = new Map<number, WsStub>();
	#hostTokens = new Map<number, TokenStub>();
	/** Child-created tokens use negative ids to avoid clashing with host ids. */
	#nextChildTokenId = -1;
	#nextRegionId = 1;

	#runtimeState: object = {};
	#abortController = new AbortController();

	/** Callbacks bag captured by createActorFactory. */
	callbacks: Record<string, unknown> | undefined;

	constructor(port: MessagePort, ctxMeta: BridgeCtxMeta) {
		this.#port = port;
		this.#ctxMeta = ctxMeta;
	}

	get ctxStub(): ActorContextHandle {
		return this.#ctxStub as unknown as ActorContextHandle;
	}

	// MARK: Transport

	#send(message: ChildToHostMessage) {
		this.#port.postMessage(message);
	}

	#call(method: string, args: unknown[]): Promise<unknown> {
		const seq = this.#nextSeq++;
		if (process.env.RIVETKIT_BRIDGE_DEBUG === "1") {
			logger().warn({ msg: "bridge rpc:call", seq, method });
		}
		return new Promise((resolve, reject) => {
			this.#pending.set(seq, { resolve, reject });
			this.#send({
				kind: "rpc:call",
				seq,
				method,
				args: args.map((arg) => this.#encodeArg(arg)),
			});
		});
	}

	#post(method: string, args: unknown[]) {
		this.#send({
			kind: "rpc:post",
			method,
			args: args.map((arg) => this.#encodeArg(arg)),
		});
	}

	#sync(method: string, args: unknown[]): unknown {
		const encodedArgs = args.map((arg) => this.#encodeArg(arg));
		const result = this.#syncCaller.call((sab) => {
			this.#send({ kind: "rpc:sync", method, args: encodedArgs, sab });
		});
		if (!result.ok) {
			throw fromBridgeErrorPayload(
				result.error ?? { message: "sync rpc failed" },
			);
		}
		return this.#decodeResult(result.value);
	}

	/** Handle host -> child rpc results and pushed events. */
	handleRpcResult(
		seq: number,
		ok: boolean,
		value?: unknown,
		error?: BridgeErrorPayload,
	) {
		const pending = this.#pending.get(seq);
		if (!pending) {
			logger().warn({ msg: "rpc result for unknown seq", seq });
			return;
		}
		this.#pending.delete(seq);
		if (process.env.RIVETKIT_BRIDGE_DEBUG === "1") {
			logger().warn({ msg: "bridge rpc:result", seq, ok });
		}
		if (ok) {
			pending.resolve(this.#decodeResult(value));
		} else {
			pending.reject(
				fromBridgeErrorPayload(error ?? { message: "rpc failed" }),
			);
		}
	}

	handleWebSocketEvent(wsId: number, event: RuntimeWebSocketEvent) {
		const stub = this.#wsStubs.get(wsId);
		if (!stub) {
			logger().debug({
				msg: "websocket event for unknown handle",
				wsId,
				event: event.kind,
			});
			return;
		}
		if (stub.callback) {
			stub.callback(event);
		} else {
			stub.buffered.push(event);
		}
		if (event.kind === "close") {
			this.#wsStubs.delete(wsId);
		}
	}

	handleTokenCancelled(tokenId: number) {
		const stub = this.#hostTokens.get(tokenId);
		if (!stub || stub.aborted) {
			return;
		}
		stub.aborted = true;
		for (const listener of stub.listeners.splice(0)) {
			try {
				listener();
			} catch (error) {
				logger().warn({
					msg: "cancellation listener failed",
					error: toBridgeErrorPayload(error).message,
				});
			}
		}
	}

	handleAbort() {
		this.#abortController.abort();
	}

	/** Fail every in-flight rpc; called when the host port closes. */
	failPending(reason: string) {
		const pending = Array.from(this.#pending.values());
		this.#pending.clear();
		for (const deferred of pending) {
			deferred.reject(new Error(reason));
		}
	}

	// MARK: Arg / result translation

	#encodeArg(value: unknown): unknown {
		if (value instanceof CtxStub) {
			return { __bridge: "ctx" } satisfies BridgeHandleRef;
		}
		if (value instanceof ConnStub) {
			return {
				__bridge: "conn",
				connId: value.connId,
			} satisfies BridgeHandleRef;
		}
		if (value instanceof WsStub) {
			return { __bridge: "ws", id: value.id } satisfies BridgeHandleRef;
		}
		if (value instanceof TokenStub) {
			return {
				__bridge: "token",
				id: value.id,
				aborted: value.aborted,
			} satisfies BridgeHandleRef;
		}
		return value;
	}

	#decodeResult(value: unknown): unknown {
		if (Array.isArray(value)) {
			return value.map((entry) => this.#decodeResult(entry));
		}
		if (isBridgeHandleRef(value)) {
			return this.decodeHandleRef(value);
		}
		if (
			typeof value === "object" &&
			value !== null &&
			"__bridgeQueueMessage" in value
		) {
			return new QueueMessageStub(
				(value as { __bridgeQueueMessage: BridgeQueueMessage })
					.__bridgeQueueMessage,
				this,
			);
		}
		return value;
	}

	/** Resolve a pushed handle ref to its child stub, creating it on demand. */
	decodeHandleRef(ref: BridgeHandleRef): unknown {
		switch (ref.__bridge) {
			case "ctx":
				return this.#ctxStub;
			case "conn": {
				let stub = this.#conns.get(ref.connId);
				if (!stub) {
					if (!ref.meta) {
						throw new Error(
							`conn ${ref.connId} crossed the bridge without metadata`,
						);
					}
					stub = new ConnStub(
						ref.meta.connId,
						ref.meta.params,
						ref.meta.isHibernatable,
						ref.meta.state,
					);
					this.#conns.set(ref.connId, stub);
				}
				return stub;
			}
			case "ws": {
				let stub = this.#wsStubs.get(ref.id);
				if (!stub) {
					stub = new WsStub(ref.id);
					this.#wsStubs.set(ref.id, stub);
				}
				return stub;
			}
			case "token": {
				let stub = this.#hostTokens.get(ref.id);
				if (!stub) {
					stub = new TokenStub(ref.id);
					stub.aborted = ref.aborted === true;
					this.#hostTokens.set(ref.id, stub);
				}
				return stub;
			}
		}
	}

	/** Drop a conn stub once the host reports the conn fully disconnected. */
	releaseConn(connId: string) {
		this.#conns.delete(connId);
	}

	async completeQueueMessage(
		completableId: number,
		response?: RuntimeBytes,
	): Promise<void> {
		await this.#call("queueMessageComplete", [completableId, response]);
	}

	#asConn(conn: ConnHandle): ConnStub {
		if (!(conn instanceof ConnStub)) {
			throw new Error("expected bridged conn handle");
		}
		return conn;
	}

	#asWs(ws: WebSocketHandle): WsStub {
		if (!(ws instanceof WsStub)) {
			throw new Error("expected bridged websocket handle");
		}
		return ws;
	}

	#asToken(token: CancellationTokenHandle): TokenStub {
		if (!(token instanceof TokenStub)) {
			throw new Error("expected bridged cancellation token handle");
		}
		return token;
	}

	#region(api: BridgeRegionApi, promise: Promise<unknown>): number {
		const regionId = this.#nextRegionId++;
		this.#send({ kind: "region:begin", regionId, api });
		promise.then(
			() => this.#send({ kind: "region:end", regionId, api }),
			(error) =>
				this.#send({
					kind: "region:end",
					regionId,
					api,
					error: toBridgeErrorPayload(error),
				}),
		);
		return regionId;
	}

	// MARK: Registry surface (host-only)

	createRegistry(): RegistryHandle {
		throw registryMethodError("createRegistry");
	}
	registerActor(): void {
		throw registryMethodError("registerActor");
	}
	serveRegistry(): Promise<void> {
		throw registryMethodError("serveRegistry");
	}
	shutdownRegistry(): Promise<void> {
		throw registryMethodError("shutdownRegistry");
	}
	handleServerlessRequest(): never {
		throw registryMethodError("handleServerlessRequest");
	}

	createActorFactory(
		callbacks: object,
		_config?: RuntimeActorConfig | undefined | null,
	): ActorFactoryHandle {
		this.callbacks = callbacks as Record<string, unknown>;
		return { bridgeKind: "factory" } as unknown as ActorFactoryHandle;
	}

	// MARK: Cancellation tokens

	createCancellationToken(): CancellationTokenHandle {
		const stub = new TokenStub(this.#nextChildTokenId--);
		this.#hostTokens.set(stub.id, stub);
		this.#post("tokenCreate", [stub.id]);
		return stub as unknown as CancellationTokenHandle;
	}

	cancellationTokenAborted(token: CancellationTokenHandle): boolean {
		return this.#asToken(token).aborted;
	}

	cancelCancellationToken(token: CancellationTokenHandle): void {
		const stub = this.#asToken(token);
		if (stub.aborted) {
			return;
		}
		stub.aborted = true;
		this.#post("tokenCancel", [stub.id]);
		for (const listener of stub.listeners.splice(0)) {
			listener();
		}
	}

	onCancellationTokenCancelled(
		token: CancellationTokenHandle,
		callback: (...args: unknown[]) => unknown,
	): void {
		const stub = this.#asToken(token);
		if (stub.aborted) {
			callback();
			return;
		}
		stub.listeners.push(callback);
	}

	// MARK: Actor context

	actorState(_ctx: ActorContextHandle): RuntimeBytes {
		return (this.#sync("actorState", [this.#ctxStub]) ??
			new Uint8Array()) as RuntimeBytes;
	}

	actorBeginOnStateChange(_ctx: ActorContextHandle): void {
		this.#post("actorBeginOnStateChange", [this.#ctxStub]);
	}

	actorEndOnStateChange(_ctx: ActorContextHandle): void {
		this.#post("actorEndOnStateChange", [this.#ctxStub]);
	}

	actorSetAlarm(
		_ctx: ActorContextHandle,
		timestampMs?: number | undefined | null,
	): void {
		this.#post("actorSetAlarm", [this.#ctxStub, timestampMs]);
	}

	actorRequestSave(
		_ctx: ActorContextHandle,
		opts?: RuntimeRequestSaveOpts | undefined | null,
	): void {
		this.#post("actorRequestSave", [this.#ctxStub, opts]);
	}

	async actorRequestSaveAndWait(
		_ctx: ActorContextHandle,
		opts?: RuntimeRequestSaveOpts | undefined | null,
	): Promise<void> {
		await this.#call("actorRequestSaveAndWait", [this.#ctxStub, opts]);
	}

	actorInspectorSnapshot(_ctx: ActorContextHandle): RuntimeInspectorSnapshot {
		return this.#sync("actorInspectorSnapshot", [
			this.#ctxStub,
		]) as RuntimeInspectorSnapshot;
	}

	actorDecodeInspectorRequest(
		_ctx: ActorContextHandle,
		bytes: RuntimeBytes,
		advertisedVersion: number,
	): RuntimeBytes {
		return this.#sync("actorDecodeInspectorRequest", [
			this.#ctxStub,
			bytes,
			advertisedVersion,
		]) as RuntimeBytes;
	}

	actorEncodeInspectorResponse(
		_ctx: ActorContextHandle,
		bytes: RuntimeBytes,
		targetVersion: number,
	): RuntimeBytes {
		return this.#sync("actorEncodeInspectorResponse", [
			this.#ctxStub,
			bytes,
			targetVersion,
		]) as RuntimeBytes;
	}

	async actorVerifyInspectorAuth(
		_ctx: ActorContextHandle,
		bearerToken?: string | undefined | null,
	): Promise<void> {
		await this.#call("actorVerifyInspectorAuth", [
			this.#ctxStub,
			bearerToken,
		]);
	}

	actorQueueHibernationRemoval(
		_ctx: ActorContextHandle,
		connId: string,
	): void {
		this.#post("actorQueueHibernationRemoval", [this.#ctxStub, connId]);
	}

	actorTakePendingHibernationChanges(_ctx: ActorContextHandle): string[] {
		return this.#sync("actorTakePendingHibernationChanges", [
			this.#ctxStub,
		]) as string[];
	}

	actorDirtyHibernatableConns(_ctx: ActorContextHandle): ConnHandle[] {
		return this.#sync("actorDirtyHibernatableConns", [
			this.#ctxStub,
		]) as ConnHandle[];
	}

	async actorSaveState(
		_ctx: ActorContextHandle,
		payload: RuntimeStateDeltaPayload,
	): Promise<void> {
		await this.#call("actorSaveState", [this.#ctxStub, payload]);
	}

	actorId(_ctx: ActorContextHandle): string {
		return this.#ctxMeta.actorId;
	}

	actorName(_ctx: ActorContextHandle): string {
		return this.#ctxMeta.actorName;
	}

	actorKey(_ctx: ActorContextHandle): RuntimeActorKeySegment[] {
		return this.#ctxMeta.actorKey;
	}

	actorRegion(_ctx: ActorContextHandle): string {
		return this.#ctxMeta.actorRegion;
	}

	actorSleep(_ctx: ActorContextHandle): void {
		this.#post("actorSleep", [this.#ctxStub]);
	}

	actorDestroy(_ctx: ActorContextHandle): void {
		this.#post("actorDestroy", [this.#ctxStub]);
	}

	actorAbortSignal(_ctx: ActorContextHandle): AbortSignal {
		return this.#abortController.signal;
	}

	actorConns(_ctx: ActorContextHandle): ConnHandle[] {
		return this.#sync("actorConns", [this.#ctxStub]) as ConnHandle[];
	}

	async actorConnectConn(
		_ctx: ActorContextHandle,
		params: RuntimeBytes,
		request?: unknown,
	): Promise<ConnHandle> {
		return (await this.#call("actorConnectConn", [
			this.#ctxStub,
			params,
			request,
		])) as ConnHandle;
	}

	actorBroadcast(
		_ctx: ActorContextHandle,
		name: string,
		args: RuntimeBytes,
	): void {
		this.#post("actorBroadcast", [this.#ctxStub, name, args]);
	}

	actorWaitUntil(_ctx: ActorContextHandle, promise: Promise<unknown>): void {
		this.#region("waitUntil", promise);
	}

	async actorWaitForTrackedShutdownWork(
		_ctx: ActorContextHandle,
	): Promise<boolean> {
		return (await this.#call("actorWaitForTrackedShutdownWork", [
			this.#ctxStub,
		])) as boolean;
	}

	async actorWaitForTrackedShutdownWorkUnbounded(
		_ctx: ActorContextHandle,
	): Promise<void> {
		await this.#call("actorWaitForTrackedShutdownWorkUnbounded", [
			this.#ctxStub,
		]);
	}

	actorKeepAwake(_ctx: ActorContextHandle, promise: Promise<unknown>): void {
		this.#region("keepAwake", promise);
	}

	actorBeginKeepAwake(_ctx: ActorContextHandle): number {
		const regionId = this.#nextRegionId++;
		this.#send({ kind: "region:begin", regionId, api: "beginKeepAwake" });
		return regionId;
	}

	actorEndKeepAwake(_ctx: ActorContextHandle, regionId: number): void {
		this.#send({ kind: "region:end", regionId, api: "beginKeepAwake" });
	}

	actorRegisterTask(
		_ctx: ActorContextHandle,
		promise: Promise<unknown>,
	): void {
		this.#region("registerTask", promise);
	}

	actorRuntimeState(_ctx: ActorContextHandle): object {
		return this.#runtimeState;
	}

	actorClearRuntimeState(_ctx: ActorContextHandle): void {
		this.#runtimeState = {};
		this.#post("actorClearRuntimeState", [this.#ctxStub]);
	}

	actorRestartRunHandler(_ctx: ActorContextHandle): void {
		this.#post("actorRestartRunHandler", [this.#ctxStub]);
	}

	actorBeginWebsocketCallback(_ctx: ActorContextHandle): number {
		const regionId = this.#nextRegionId++;
		this.#send({
			kind: "region:begin",
			regionId,
			api: "beginWebsocketCallback",
		});
		return regionId;
	}

	actorEndWebsocketCallback(
		_ctx: ActorContextHandle,
		regionId: number,
	): void {
		this.#send({
			kind: "region:end",
			regionId,
			api: "beginWebsocketCallback",
		});
	}

	// MARK: KV

	async actorKvGet(
		_ctx: ActorContextHandle,
		key: RuntimeBytes,
	): Promise<RuntimeBytes | null> {
		return (await this.#call("actorKvGet", [
			this.#ctxStub,
			key,
		])) as RuntimeBytes | null;
	}

	async actorKvPut(
		_ctx: ActorContextHandle,
		key: RuntimeBytes,
		value: RuntimeBytes,
	): Promise<void> {
		await this.#call("actorKvPut", [this.#ctxStub, key, value]);
	}

	async actorKvDelete(
		_ctx: ActorContextHandle,
		key: RuntimeBytes,
	): Promise<void> {
		await this.#call("actorKvDelete", [this.#ctxStub, key]);
	}

	async actorKvDeleteRange(
		_ctx: ActorContextHandle,
		start: RuntimeBytes,
		end: RuntimeBytes,
	): Promise<void> {
		await this.#call("actorKvDeleteRange", [this.#ctxStub, start, end]);
	}

	async actorKvListPrefix(
		_ctx: ActorContextHandle,
		prefix: RuntimeBytes,
		options?: RuntimeKvListOptions | undefined | null,
	): Promise<RuntimeKvEntry[]> {
		return (await this.#call("actorKvListPrefix", [
			this.#ctxStub,
			prefix,
			options,
		])) as RuntimeKvEntry[];
	}

	async actorKvListRange(
		_ctx: ActorContextHandle,
		start: RuntimeBytes,
		end: RuntimeBytes,
		options?: RuntimeKvListOptions | undefined | null,
	): Promise<RuntimeKvEntry[]> {
		return (await this.#call("actorKvListRange", [
			this.#ctxStub,
			start,
			end,
			options,
		])) as RuntimeKvEntry[];
	}

	async actorKvBatchGet(
		_ctx: ActorContextHandle,
		keys: RuntimeBytes[],
	): Promise<Array<RuntimeBytes | undefined | null>> {
		return (await this.#call("actorKvBatchGet", [
			this.#ctxStub,
			keys,
		])) as Array<RuntimeBytes | undefined | null>;
	}

	async actorKvBatchPut(
		_ctx: ActorContextHandle,
		entries: RuntimeKvEntry[],
	): Promise<void> {
		await this.#call("actorKvBatchPut", [this.#ctxStub, entries]);
	}

	async actorKvBatchDelete(
		_ctx: ActorContextHandle,
		keys: RuntimeBytes[],
	): Promise<void> {
		await this.#call("actorKvBatchDelete", [this.#ctxStub, keys]);
	}

	// MARK: SQL

	async actorSqlExec(
		_ctx: ActorContextHandle,
		sql: string,
	): Promise<RuntimeSqlExecResult> {
		return (await this.#call("actorSqlExec", [
			this.#ctxStub,
			sql,
		])) as RuntimeSqlExecResult;
	}

	async actorSqlExecute(
		_ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlExecuteResult> {
		return (await this.#call("actorSqlExecute", [
			this.#ctxStub,
			sql,
			params,
		])) as RuntimeSqlExecuteResult;
	}

	async actorSqlQuery(
		_ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlQueryResult> {
		return (await this.#call("actorSqlQuery", [
			this.#ctxStub,
			sql,
			params,
		])) as RuntimeSqlQueryResult;
	}

	async actorSqlRun(
		_ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlRunResult> {
		return (await this.#call("actorSqlRun", [
			this.#ctxStub,
			sql,
			params,
		])) as RuntimeSqlRunResult;
	}

	actorSqlMetrics(_ctx: ActorContextHandle) {
		return this.#sync("actorSqlMetrics", [this.#ctxStub]) as ReturnType<
			CoreRuntime["actorSqlMetrics"]
		>;
	}

	actorSqlTakeLastKvError(_ctx: ActorContextHandle): string | null {
		return this.#sync("actorSqlTakeLastKvError", [this.#ctxStub]) as
			| string
			| null;
	}

	async actorSqlClose(_ctx: ActorContextHandle): Promise<void> {
		await this.#call("actorSqlClose", [this.#ctxStub]);
	}

	// MARK: Queue

	async actorQueueSend(
		_ctx: ActorContextHandle,
		name: string,
		body: RuntimeBytes,
	): Promise<RuntimeQueueMessage> {
		return (await this.#call("actorQueueSend", [
			this.#ctxStub,
			name,
			body,
		])) as RuntimeQueueMessage;
	}

	async actorQueueNextBatch(
		_ctx: ActorContextHandle,
		options?: RuntimeQueueNextBatchOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeQueueMessage[]> {
		return (await this.#call("actorQueueNextBatch", [
			this.#ctxStub,
			options,
			signal,
		])) as RuntimeQueueMessage[];
	}

	async actorQueueWaitForNames(
		_ctx: ActorContextHandle,
		names: string[],
		options?: RuntimeQueueWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeQueueMessage> {
		return (await this.#call("actorQueueWaitForNames", [
			this.#ctxStub,
			names,
			options,
			signal,
		])) as RuntimeQueueMessage;
	}

	async actorQueueWaitForNamesAvailable(
		_ctx: ActorContextHandle,
		names: string[],
		options?: RuntimeQueueWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<void> {
		await this.#call("actorQueueWaitForNamesAvailable", [
			this.#ctxStub,
			names,
			options,
			signal,
		]);
	}

	async actorQueueEnqueueAndWait(
		_ctx: ActorContextHandle,
		name: string,
		body: RuntimeBytes,
		options?: RuntimeQueueEnqueueAndWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeBytes | null> {
		return (await this.#call("actorQueueEnqueueAndWait", [
			this.#ctxStub,
			name,
			body,
			options,
			signal,
		])) as RuntimeBytes | null;
	}

	actorQueueTryNextBatch(
		_ctx: ActorContextHandle,
		options?: RuntimeQueueTryNextBatchOptions | undefined | null,
	): RuntimeQueueMessage[] {
		return this.#sync("actorQueueTryNextBatch", [
			this.#ctxStub,
			options,
		]) as RuntimeQueueMessage[];
	}

	actorQueueMaxSize(_ctx: ActorContextHandle): number {
		return this.#ctxMeta.queueMaxSize;
	}

	async actorQueueInspectMessages(
		_ctx: ActorContextHandle,
	): Promise<RuntimeQueueInspectMessage[]> {
		return (await this.#call("actorQueueInspectMessages", [
			this.#ctxStub,
		])) as RuntimeQueueInspectMessage[];
	}

	// MARK: Schedule

	actorScheduleAfter(
		_ctx: ActorContextHandle,
		durationMs: number,
		actionName: string,
		args: RuntimeBytes,
	): void {
		this.#post("actorScheduleAfter", [
			this.#ctxStub,
			durationMs,
			actionName,
			args,
		]);
	}

	actorScheduleAt(
		_ctx: ActorContextHandle,
		timestampMs: number,
		actionName: string,
		args: RuntimeBytes,
	): void {
		this.#post("actorScheduleAt", [
			this.#ctxStub,
			timestampMs,
			actionName,
			args,
		]);
	}

	// MARK: Conns

	connId(conn: ConnHandle): string {
		return this.#asConn(conn).connId;
	}

	connParams(conn: ConnHandle): RuntimeBytes {
		return this.#asConn(conn).params;
	}

	connState(conn: ConnHandle): RuntimeBytes {
		return this.#asConn(conn).state;
	}

	connSetState(conn: ConnHandle, state: RuntimeBytes): void {
		const stub = this.#asConn(conn);
		// The child is the only conn-state writer, so the local mirror stays
		// authoritative between host pushes.
		stub.state = state;
		this.#post("connSetState", [stub, state]);
	}

	connIsHibernatable(conn: ConnHandle): boolean {
		return this.#asConn(conn).isHibernatable;
	}

	connSend(conn: ConnHandle, name: string, args: RuntimeBytes): void {
		this.#post("connSend", [this.#asConn(conn), name, args]);
	}

	async connDisconnect(
		conn: ConnHandle,
		reason?: string | undefined | null,
	): Promise<void> {
		await this.#call("connDisconnect", [this.#asConn(conn), reason]);
	}

	// MARK: WebSockets

	webSocketSend(ws: WebSocketHandle, data: RuntimeBytes, binary: boolean) {
		this.#post("webSocketSend", [this.#asWs(ws), data, binary]);
	}

	async webSocketClose(
		ws: WebSocketHandle,
		code?: number | undefined | null,
		reason?: string | undefined | null,
	): Promise<void> {
		await this.#call("webSocketClose", [this.#asWs(ws), code, reason]);
	}

	webSocketSetEventCallback(
		ws: WebSocketHandle,
		callback: (event: RuntimeWebSocketEvent) => void,
	): void {
		const stub = this.#asWs(ws);
		stub.callback = callback;
		for (const event of stub.buffered.splice(0)) {
			callback(event);
		}
	}
}
