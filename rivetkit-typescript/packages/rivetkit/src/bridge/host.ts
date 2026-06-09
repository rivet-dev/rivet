import { createRequire } from "node:module";
import { Worker } from "node:worker_threads";
import { getLogger } from "@/common/log";
import { encodeValue } from "@/registry/native";
import type {
	ActorContextHandle,
	ActorFactoryHandle,
	CancellationTokenHandle,
	ConnHandle,
	CoreRuntime,
	RuntimeActorConfig,
	RuntimeQueueMessage,
	RuntimeWebSocketEvent,
	WebSocketHandle,
} from "@/registry/runtime";
import { fromBridgeErrorPayload, toBridgeErrorPayload } from "./errors";
import {
	BRIDGE_ASYNC_METHODS,
	BRIDGE_POST_METHODS,
	BRIDGE_SYNC_METHODS,
	type BridgeBootstrap,
	type BridgeConnMeta,
	type BridgeCtxMeta,
	type BridgeHandleRef,
	type BridgeQueueMessage,
	type BridgeRegionApi,
	type BridgeRegistryConfig,
	type ChildToHostMessage,
	type HostToChildMessage,
	isBridgeHandleRef,
} from "./protocol";
import { respondSync } from "./sync-channel";

function logger() {
	return getLogger("actor-bridge-host");
}

/** How long dispose waits for in-flight envelopes before terminating. */
const DISPOSE_DRAIN_TIMEOUT_MS = 5_000;

export interface BridgeSpawnPlan {
	bootstrap: BridgeBootstrap;
	/** Prebundled entry path when running from TypeScript source. */
	devBundlePath?: string;
}

export interface BridgeDescriptor {
	/** Resolves the child spawn plan once the actor identity is known. */
	resolveSpawn: (info: {
		actorId: string;
		key: string[];
	}) => Promise<BridgeSpawnPlan> | BridgeSpawnPlan;
	registryConfig: BridgeRegistryConfig;
	actorName: string;
}

interface Deferred<T = unknown> {
	promise: Promise<T>;
	resolve: (value: T) => void;
	reject: (error: unknown) => void;
}

function deferred<T = unknown>(): Deferred<T> {
	let resolve!: (value: T) => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	return { promise, resolve, reject };
}

/** True when the host runs from TypeScript source (vitest, tsx). */
export function runningFromSource(): boolean {
	return import.meta.url.endsWith(".ts");
}

/**
 * Resolve the worker entry for the bridge child.
 *
 * Built packages reference the bundled JS entry through the package
 * self-reference export and the child resolves its definition with runtime
 * imports. When running from TypeScript source, the caller prebundles a
 * per-definition entry with esbuild (see dev-bundle.ts) and passes its path.
 */
function resolveChildEntry(devBundlePath: string | undefined): string {
	if (devBundlePath) {
		return devBundlePath;
	}
	if (runningFromSource()) {
		throw new Error(
			"bridge child entry requires a dev bundle when running from TypeScript source",
		);
	}
	const require = createRequire(import.meta.url);
	return require.resolve("rivetkit/bridge-child");
}

/** Promise-region bookkeeping for promise-argument runtime APIs. */
interface RegionEntry {
	api: BridgeRegionApi;
	deferred?: Deferred<unknown>;
	realRegionId?: number;
}

/**
 * One bridged actor child: a worker thread serving a single actor instance.
 *
 * Owns the handle tables that map wire refs to real runtime handles, the
 * pending-call maps in both directions, and the worker lifecycle. The child
 * is identified with one actor generation; wakes after sleep spawn a fresh
 * instance (detected through the actor runtime-state bag reset).
 */
class BridgedActorChild {
	#runtime: CoreRuntime;
	#worker: Worker;
	#ready: Deferred<void>;
	#disposed = false;

	/** Latest real ctx handle observed from a callback payload. */
	#ctx: ActorContextHandle;
	#abortHooked = false;

	#nextCbSeq = 1;
	#pendingCallbacks = new Map<number, Deferred>();
	#callbackNames = new Set<string>();

	#conns = new Map<string, ConnHandle>();
	#connsSentToChild = new Set<string>();
	#nextWsId = 1;
	#wsHandles = new Map<number, WebSocketHandle>();
	#nextTokenId = 1;
	#tokens = new Map<number, CancellationTokenHandle>();
	#nextQueueMessageId = 1;
	#queueMessages = new Map<number, RuntimeQueueMessage>();
	#regions = new Map<number, RegionEntry>();

	constructor(
		runtime: CoreRuntime,
		ctx: ActorContextHandle,
		bootstrap: BridgeBootstrap,
		registryConfig: BridgeRegistryConfig,
		actorName: string,
		actorId: string,
		devBundlePath: string | undefined,
	) {
		this.#runtime = runtime;
		this.#ctx = ctx;
		this.#ready = deferred<void>();

		const ctxMeta: BridgeCtxMeta = {
			actorId,
			actorName,
			actorKey: runtime.actorKey(ctx),
			actorRegion: runtime.actorRegion(ctx),
			queueMaxSize: runtime.actorQueueMaxSize(ctx),
		};

		this.#worker = new Worker(resolveChildEntry(devBundlePath), {
			workerData: {
				bootstrap,
				registryConfig,
				actorName,
				actorId,
				ctxMeta,
			},
			resourceLimits:
				bootstrap.kind === "source"
					? bootstrap.workerResourceLimits
					: undefined,
		});

		this.#worker.on("message", (message: ChildToHostMessage) => {
			this.#handleMessage(message);
		});
		this.#worker.on("error", (error) => {
			logger().error({
				msg: "bridge child worker errored",
				actorId,
				error: error.message,
			});
			this.#failAll(error);
		});
		this.#worker.on("exit", (code) => {
			if (!this.#disposed) {
				this.#failAll(
					new Error(
						`bridged actor worker exited unexpectedly with code ${code}`,
					),
				);
			}
		});
	}

	get ready(): Promise<void> {
		return this.#ready.promise;
	}

	/**
	 * Whether the loaded definition registered the named callback. Only
	 * meaningful after ready resolves; callers short-circuit absent callbacks
	 * to their no-op defaults without a child round trip.
	 */
	hasCallback(name: string): boolean {
		return this.#callbackNames.has(name);
	}

	/** Track the freshest real ctx handle and hook the abort signal once. */
	observeCtx(ctx: ActorContextHandle) {
		this.#ctx = ctx;
		if (!this.#abortHooked) {
			this.#abortHooked = true;
			const signal = this.#runtime.actorAbortSignal(ctx);
			if (signal.aborted) {
				this.#send({ kind: "evt:abort" });
			} else {
				signal.addEventListener(
					"abort",
					() => this.#send({ kind: "evt:abort" }),
					{ once: true },
				);
			}
		}
	}

	#send(message: HostToChildMessage) {
		this.#worker.postMessage(message);
	}

	#failAll(error: unknown) {
		this.#ready.reject(error);
		// Avoid unhandled rejection when the failure happens after startup.
		this.#ready.promise.catch(() => {});
		const pending = Array.from(this.#pendingCallbacks.values());
		this.#pendingCallbacks.clear();
		for (const entry of pending) {
			entry.reject(error);
		}
		for (const [, region] of this.#regions) {
			region.deferred?.resolve(null);
		}
		this.#regions.clear();
	}

	async invokeCallback(
		callback: string,
		actionName: string | undefined,
		payload: Record<string, unknown>,
	): Promise<unknown> {
		await this.#ready.promise;
		if (this.#disposed) {
			throw new Error(
				`bridged actor child is disposed; cannot dispatch ${callback}`,
			);
		}
		const seq = this.#nextCbSeq++;
		const entry = deferred();
		this.#pendingCallbacks.set(seq, entry);
		this.#send({
			kind: "cb:invoke",
			seq,
			callback,
			actionName,
			payload: this.#encodePayload(payload),
		});
		return await entry.promise;
	}

	/** Drop host-side conn bookkeeping once a conn fully disconnected. */
	releaseConn(conn: ConnHandle) {
		const connId = this.#runtime.connId(conn);
		this.#conns.delete(connId);
		this.#connsSentToChild.delete(connId);
	}

	hasPendingWork(): boolean {
		return this.#pendingCallbacks.size > 0;
	}

	async dispose(reason: string): Promise<void> {
		if (this.#disposed) {
			return;
		}
		this.#disposed = true;
		logger().debug({ msg: "disposing bridge child", reason });

		// Give in-flight envelopes one bounded chance to settle. New
		// invocations are rejected once disposed, so the pending set only
		// shrinks; long-lived callbacks (run handlers) settle when the core
		// aborts their work during shutdown.
		if (this.#pendingCallbacks.size > 0) {
			const pending = Array.from(
				this.#pendingCallbacks.values(),
				(entry) => entry.promise.catch(() => {}),
			);
			let drainTimeout: ReturnType<typeof setTimeout> | undefined;
			await Promise.race([
				Promise.all(pending),
				new Promise((resolve) => {
					drainTimeout = setTimeout(
						resolve,
						DISPOSE_DRAIN_TIMEOUT_MS,
					);
				}),
			]);
			if (drainTimeout !== undefined) {
				clearTimeout(drainTimeout);
			}
		}

		this.#failAll(new Error("bridged actor child disposed"));
		await this.#worker.terminate();
	}

	// MARK: Payload translation

	#encodePayload(payload: Record<string, unknown>): Record<string, unknown> {
		const encoded: Record<string, unknown> = {};
		for (const [key, value] of Object.entries(payload)) {
			if (value === null || value === undefined) {
				encoded[key] = value;
				continue;
			}
			switch (key) {
				case "ctx":
					this.observeCtx(value as ActorContextHandle);
					encoded[key] = {
						__bridge: "ctx",
					} satisfies BridgeHandleRef;
					break;
				case "conn":
					encoded[key] = this.#encodeConn(value as ConnHandle);
					break;
				case "ws":
					encoded[key] = this.#encodeWs(value as WebSocketHandle);
					break;
				case "cancelToken":
					encoded[key] = this.#encodeToken(
						value as CancellationTokenHandle,
					);
					break;
				default:
					encoded[key] = value;
			}
		}
		return encoded;
	}

	#encodeConn(conn: ConnHandle): BridgeHandleRef {
		const connId = this.#runtime.connId(conn);
		this.#conns.set(connId, conn);
		if (this.#connsSentToChild.has(connId)) {
			return { __bridge: "conn", connId };
		}
		this.#connsSentToChild.add(connId);
		const meta: BridgeConnMeta = {
			connId,
			params: this.#runtime.connParams(conn),
			isHibernatable: this.#runtime.connIsHibernatable(conn),
			state: this.#runtime.connState(conn),
		};
		return { __bridge: "conn", connId, meta };
	}

	#encodeWs(ws: WebSocketHandle): BridgeHandleRef {
		const id = this.#nextWsId++;
		this.#wsHandles.set(id, ws);
		// Register the forwarder before the child sees the handle so no event
		// can be lost; the child buffers events until the glue registers its
		// callback.
		this.#runtime.webSocketSetEventCallback(
			ws,
			(event: RuntimeWebSocketEvent) => {
				this.#send({ kind: "evt:websocket", wsId: id, event });
				if (event.kind === "close") {
					this.#wsHandles.delete(id);
				}
			},
		);
		return { __bridge: "ws", id };
	}

	#encodeToken(token: CancellationTokenHandle): BridgeHandleRef {
		const id = this.#nextTokenId++;
		this.#tokens.set(id, token);
		const aborted = this.#runtime.cancellationTokenAborted(token);
		if (!aborted) {
			this.#runtime.onCancellationTokenCancelled(token, () => {
				this.#send({ kind: "evt:tokenCancelled", tokenId: id });
			});
		}
		return { __bridge: "token", id, aborted };
	}

	#encodeQueueMessage(message: RuntimeQueueMessage): {
		__bridgeQueueMessage: BridgeQueueMessage;
	} {
		let completableId: number | undefined;
		if (message.isCompletable()) {
			completableId = this.#nextQueueMessageId++;
			this.#queueMessages.set(completableId, message);
		}
		return {
			__bridgeQueueMessage: {
				id: message.id(),
				name: message.name(),
				body: message.body(),
				createdAt: message.createdAt(),
				completableId,
			},
		};
	}

	#decodeArg(value: unknown): unknown {
		if (!isBridgeHandleRef(value)) {
			return value;
		}
		switch (value.__bridge) {
			case "ctx":
				return this.#ctx;
			case "conn": {
				const conn = this.#conns.get(value.connId);
				if (!conn) {
					throw new Error(
						`unknown bridged conn handle ${value.connId}`,
					);
				}
				return conn;
			}
			case "ws": {
				const ws = this.#wsHandles.get(value.id);
				if (!ws) {
					throw new Error(
						`unknown bridged websocket handle ${value.id}`,
					);
				}
				return ws;
			}
			case "token": {
				let token = this.#tokens.get(value.id);
				if (!token) {
					// Child-created tokens materialize lazily on first use.
					token = this.#runtime.createCancellationToken();
					if (value.aborted) {
						this.#runtime.cancelCancellationToken(token);
					}
					this.#tokens.set(value.id, token);
				}
				return token;
			}
		}
	}

	#encodeRpcResult(method: string, value: unknown): unknown {
		switch (method) {
			case "actorConnectConn":
				return this.#encodeConn(value as ConnHandle);
			case "actorConns":
			case "actorDirtyHibernatableConns":
				return (value as ConnHandle[]).map((conn) =>
					this.#encodeConn(conn),
				);
			case "actorQueueSend":
			case "actorQueueWaitForNames":
				return this.#encodeQueueMessage(value as RuntimeQueueMessage);
			case "actorQueueNextBatch":
			case "actorQueueTryNextBatch":
				return (value as RuntimeQueueMessage[]).map((message) =>
					this.#encodeQueueMessage(message),
				);
			default:
				return value;
		}
	}

	// MARK: Child message handling

	#handleMessage(message: ChildToHostMessage) {
		switch (message.kind) {
			case "ready":
				this.#callbackNames = new Set(message.callbackNames);
				this.#ready.resolve();
				break;
			case "bootstrapError":
				this.#ready.reject(fromBridgeErrorPayload(message.error));
				this.#ready.promise.catch(() => {});
				break;
			case "cb:result": {
				const entry = this.#pendingCallbacks.get(message.seq);
				if (!entry) {
					logger().warn({
						msg: "callback result for unknown seq",
						seq: message.seq,
					});
					break;
				}
				this.#pendingCallbacks.delete(message.seq);
				if (message.ok) {
					entry.resolve(message.value);
				} else {
					entry.reject(
						fromBridgeErrorPayload(
							message.error ?? { message: "callback failed" },
						),
					);
				}
				break;
			}
			case "rpc:call":
				void this.#handleRpcCall(
					message.seq,
					message.method,
					message.args,
				);
				break;
			case "rpc:post":
				this.#handleRpcPost(message.method, message.args);
				break;
			case "rpc:sync":
				this.#handleRpcSync(message.method, message.args, message.sab);
				break;
			case "region:begin":
				this.#handleRegionBegin(message.regionId, message.api);
				break;
			case "region:end":
				this.#handleRegionEnd(
					message.regionId,
					message.api,
					message.error,
				);
				break;
		}
	}

	async #handleRpcCall(seq: number, method: string, args: unknown[]) {
		try {
			const value = await this.#invokeRuntimeMethod(method, args);
			this.#send({
				kind: "rpc:result",
				seq,
				ok: true,
				value: this.#encodeRpcResult(method, value),
			});
		} catch (error) {
			this.#send({
				kind: "rpc:result",
				seq,
				ok: false,
				error: toBridgeErrorPayload(error),
			});
		}
	}

	#handleRpcPost(method: string, args: unknown[]) {
		if (!BRIDGE_POST_METHODS.has(method)) {
			this.#send({
				kind: "evt:postError",
				method,
				error: { message: `method ${method} is not postable` },
			});
			return;
		}
		try {
			if (method === "tokenCreate") {
				const id = args[0] as number;
				const token = this.#runtime.createCancellationToken();
				this.#tokens.set(id, token);
				return;
			}
			if (method === "tokenCancel") {
				const id = args[0] as number;
				const token = this.#tokens.get(id);
				if (token) {
					this.#runtime.cancelCancellationToken(token);
				}
				return;
			}
			const runtimeMethod = (
				this.#runtime as unknown as Record<
					string,
					(...args: unknown[]) => unknown
				>
			)[method];
			runtimeMethod.apply(
				this.#runtime,
				args.map((arg) => this.#decodeArg(arg)),
			);
		} catch (error) {
			this.#send({
				kind: "evt:postError",
				method,
				error: toBridgeErrorPayload(error),
			});
		}
	}

	#handleRpcSync(method: string, args: unknown[], sab: SharedArrayBuffer) {
		if (!BRIDGE_SYNC_METHODS.has(method)) {
			respondSync(sab, {
				ok: false,
				error: {
					message: `method ${method} is not callable synchronously`,
				},
			});
			return;
		}
		try {
			const runtimeMethod = (
				this.#runtime as unknown as Record<
					string,
					(...args: unknown[]) => unknown
				>
			)[method];
			const value = runtimeMethod.apply(
				this.#runtime,
				args.map((arg) => this.#decodeArg(arg)),
			);
			respondSync(sab, {
				ok: true,
				value: this.#encodeRpcResult(method, value),
			});
		} catch (error) {
			respondSync(sab, { ok: false, error: toBridgeErrorPayload(error) });
		}
	}

	async #invokeRuntimeMethod(
		method: string,
		args: unknown[],
	): Promise<unknown> {
		if (method === "queueMessageComplete") {
			const completableId = args[0] as number;
			const response = args[1] as Uint8Array | undefined;
			const message = this.#queueMessages.get(completableId);
			if (!message) {
				throw new Error(
					`unknown completable queue message ${completableId}`,
				);
			}
			this.#queueMessages.delete(completableId);
			await message.complete(response);
			return undefined;
		}
		if (!BRIDGE_ASYNC_METHODS.has(method)) {
			throw new Error(`method ${method} is not callable over the bridge`);
		}
		const runtimeMethod = (
			this.#runtime as unknown as Record<
				string,
				(...args: unknown[]) => unknown
			>
		)[method];
		return await runtimeMethod.apply(
			this.#runtime,
			args.map((arg) => this.#decodeArg(arg)),
		);
	}

	#handleRegionBegin(regionId: number, api: BridgeRegionApi) {
		const entry: RegionEntry = { api };
		switch (api) {
			case "waitUntil":
			case "keepAwake":
			case "registerTask": {
				// Resolve with null, not undefined: the runtime bridges the
				// promise into NAPI, which cannot represent undefined.
				entry.deferred = deferred<unknown>();
				// Suppress unhandled rejections if the runtime does not chain
				// the rejected branch.
				entry.deferred.promise.catch(() => {});
				if (api === "waitUntil") {
					this.#runtime.actorWaitUntil(
						this.#ctx,
						entry.deferred.promise,
					);
				} else if (api === "keepAwake") {
					this.#runtime.actorKeepAwake(
						this.#ctx,
						entry.deferred.promise,
					);
				} else {
					this.#runtime.actorRegisterTask(
						this.#ctx,
						entry.deferred.promise,
					);
				}
				break;
			}
			case "beginKeepAwake":
				entry.realRegionId = this.#runtime.actorBeginKeepAwake(
					this.#ctx,
				);
				break;
			case "beginWebsocketCallback":
				entry.realRegionId = this.#runtime.actorBeginWebsocketCallback(
					this.#ctx,
				);
				break;
		}
		this.#regions.set(regionId, entry);
	}

	#handleRegionEnd(
		regionId: number,
		api: BridgeRegionApi,
		error?: Parameters<typeof fromBridgeErrorPayload>[0],
	) {
		const entry = this.#regions.get(regionId);
		if (!entry) {
			logger().warn({ msg: "region end without begin", regionId, api });
			return;
		}
		this.#regions.delete(regionId);
		if (entry.deferred) {
			if (error) {
				entry.deferred.reject(fromBridgeErrorPayload(error));
			} else {
				entry.deferred.resolve(null);
			}
			return;
		}
		if (entry.realRegionId !== undefined) {
			if (api === "beginKeepAwake") {
				this.#runtime.actorEndKeepAwake(this.#ctx, entry.realRegionId);
			} else {
				this.#runtime.actorEndWebsocketCallback(
					this.#ctx,
					entry.realRegionId,
				);
			}
		}
	}
}

// MARK: Factory

interface BridgeRuntimeStateBag {
	__bridgeChild?: BridgedActorChild;
}

/**
 * Build an actor factory whose callbacks proxy to a per-actor worker child.
 *
 * The real (NAPI) runtime invokes these callbacks exactly as it would the
 * in-process bag from buildNativeFactory; each invocation routes to the child
 * that owns the actor. Child spawn keys off the actor runtime-state bag so a
 * same-key recreate (which resets the bag) gets a fresh worker generation.
 */
export function buildBridgedFactory(
	runtime: CoreRuntime,
	descriptor: BridgeDescriptor,
	actorConfig: RuntimeActorConfig,
	opts: {
		/** Per-name action callbacks for definitions with known actions. */
		actionNames?: string[];
		/** Register the fallback action for unknown action sets. */
		useFallbackAction: boolean;
		/** Callback names to register; undefined registers the full surface. */
		callbackNames?: string[];
	},
): ActorFactoryHandle {
	const children = new Map<string, BridgedActorChild>();

	const ensureChild = async (
		ctx: ActorContextHandle,
	): Promise<BridgedActorChild> => {
		const bag = runtime.actorRuntimeState(ctx) as BridgeRuntimeStateBag;
		if (bag.__bridgeChild) {
			bag.__bridgeChild.observeCtx(ctx);
			return bag.__bridgeChild;
		}

		const actorId = runtime.actorId(ctx);
		// A stale child for the same actor id means the previous generation
		// ended without sleep/destroy callbacks (for example a hard crash);
		// terminate it before spawning the new generation.
		const stale = children.get(actorId);
		if (stale) {
			void stale.dispose("stale generation replaced");
		}

		const key = runtime
			.actorKey(ctx)
			.map((segment) =>
				segment.stringValue !== undefined
					? segment.stringValue
					: String(segment.numberValue ?? ""),
			);
		const spawn = await descriptor.resolveSpawn({ actorId, key });

		const child = new BridgedActorChild(
			runtime,
			ctx,
			spawn.bootstrap,
			descriptor.registryConfig,
			descriptor.actorName,
			actorId,
			spawn.devBundlePath,
		);
		bag.__bridgeChild = child;
		children.set(actorId, child);
		try {
			await child.ready;
		} catch (error) {
			children.delete(actorId);
			delete bag.__bridgeChild;
			void child.dispose("bootstrap failed");
			throw error;
		}
		return child;
	};

	// Disposal runs off the callback's critical path: the sleep/destroy reply
	// must reach the core immediately because pending long-lived callbacks
	// (run handlers) only settle once the core's shutdown sequence aborts
	// their work after the reply.
	const disposeChild = (ctx: ActorContextHandle, reason: string) => {
		const bag = runtime.actorRuntimeState(ctx) as BridgeRuntimeStateBag;
		const child = bag.__bridgeChild;
		if (!child) {
			return;
		}
		delete bag.__bridgeChild;
		for (const [actorId, entry] of children) {
			if (entry === child) {
				children.delete(actorId);
			}
		}
		child.dispose(reason).catch((error) => {
			logger().warn({
				msg: "bridge child dispose failed",
				reason,
				error: toBridgeErrorPayload(error).message,
			});
		});
	};

	// When the loaded definition did not register a callback (possible only
	// for dynamic actors, which register the full surface), mirror core's
	// unregistered-callback behavior host-side without a child round trip.
	const absentCallbackResult = (
		callback: string,
		payload: Record<string, unknown>,
	): unknown => {
		if (callback === "createState" || callback === "createConnState") {
			return encodeValue(undefined);
		}
		if (callback === "onBeforeActionResponse") {
			return payload.output;
		}
		return undefined;
	};

	const proxyCallback =
		(callback: string, callbackOpts?: { disposeAfter?: string }) =>
		async (error: unknown, payload: Record<string, unknown>) => {
			if (error !== null && error !== undefined) {
				throw error;
			}
			const ctx = payload.ctx as ActorContextHandle;
			const child = await ensureChild(ctx);
			try {
				if (!child.hasCallback(callback)) {
					return absentCallbackResult(callback, payload);
				}
				return await child.invokeCallback(callback, undefined, payload);
			} finally {
				if (callback === "onDisconnectFinal" && payload.conn) {
					child.releaseConn(payload.conn as ConnHandle);
				}
				if (callbackOpts?.disposeAfter) {
					disposeChild(ctx, callbackOpts.disposeAfter);
				}
			}
		};

	const fullSurface: Record<string, unknown> = {
		createState: proxyCallback("createState"),
		onCreate: proxyCallback("onCreate"),
		createVars: proxyCallback("createVars"),
		onMigrate: proxyCallback("onMigrate"),
		onWake: proxyCallback("onWake"),
		onBeforeActorStart: proxyCallback("onBeforeActorStart"),
		onSleep: proxyCallback("onSleep", { disposeAfter: "actor sleep" }),
		onDestroy: proxyCallback("onDestroy", {
			disposeAfter: "actor destroy",
		}),
		onBeforeConnect: proxyCallback("onBeforeConnect"),
		createConnState: proxyCallback("createConnState"),
		onConnect: proxyCallback("onConnect"),
		onDisconnectFinal: proxyCallback("onDisconnectFinal"),
		onBeforeSubscribe: proxyCallback("onBeforeSubscribe"),
		onBeforeActionResponse: proxyCallback("onBeforeActionResponse"),
		onRequest: proxyCallback("onRequest"),
		onWebSocket: proxyCallback("onWebSocket"),
		run: proxyCallback("run"),
		getWorkflowHistory: proxyCallback("getWorkflowHistory"),
		replayWorkflow: proxyCallback("replayWorkflow"),
		onQueueSend: proxyCallback("onQueueSend"),
		serializeState: proxyCallback("serializeState"),
	};

	const actionProxy =
		(name: string) =>
		async (error: unknown, payload: Record<string, unknown>) => {
			if (error !== null && error !== undefined) {
				throw error;
			}
			const ctx = payload.ctx as ActorContextHandle;
			const child = await ensureChild(ctx);
			return await child.invokeCallback("action", name, payload);
		};

	const callbacks: Record<string, unknown> = {};
	const names = opts.callbackNames ?? Object.keys(fullSurface);
	for (const name of names) {
		if (!(name in fullSurface)) {
			throw new Error(`unknown bridged callback name ${name}`);
		}
		callbacks[name] = fullSurface[name];
	}

	if (opts.actionNames) {
		callbacks.actions = Object.fromEntries(
			opts.actionNames.map((name) => [name, actionProxy(name)]),
		);
	} else {
		callbacks.actions = {};
	}
	if (opts.useFallbackAction) {
		callbacks.fallbackAction = async (
			error: unknown,
			payload: Record<string, unknown>,
		) => {
			if (error !== null && error !== undefined) {
				throw error;
			}
			const ctx = payload.ctx as ActorContextHandle;
			const child = await ensureChild(ctx);
			const actionName = payload.name as string;
			const { name: _name, ...rest } = payload;
			return await child.invokeCallback("action", actionName, rest);
		};
	}

	return runtime.createActorFactory(callbacks, actorConfig);
}

export function buildBridgeRegistryConfig(registryConfig: {
	endpoint?: string;
	token?: string;
	namespace: string;
	envoy: { poolName: string };
	headers: Record<string, string>;
	maxIncomingMessageSize: number;
	maxOutgoingMessageSize: number;
	test: { enabled: boolean };
	publicEndpoint?: string;
	publicNamespace?: string;
	publicToken?: string;
}): BridgeRegistryConfig {
	return {
		endpoint: registryConfig.endpoint,
		token: registryConfig.token,
		namespace: registryConfig.namespace,
		poolName: registryConfig.envoy.poolName,
		headers: registryConfig.headers,
		maxIncomingMessageSize: registryConfig.maxIncomingMessageSize,
		maxOutgoingMessageSize: registryConfig.maxOutgoingMessageSize,
		testEnabled: registryConfig.test.enabled,
		publicEndpoint: registryConfig.publicEndpoint,
		publicNamespace: registryConfig.publicNamespace,
		publicToken: registryConfig.publicToken,
	};
}
