import type { EnvoyConfig } from "@rivetkit/rivetkit-native/wrapper";
import type { ISqliteVfs } from "@rivetkit/sqlite-wasm";
import { SqliteVfsPoolManager } from "@/driver-helpers/sqlite-pool";
import type { HibernatingWebSocketMetadata, EnvoyHandle } from "@rivetkit/rivetkit-native/wrapper";
import type * as protocol from "@rivetkit/engine-envoy-protocol";
import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import { streamSSE } from "hono/streaming";
import { WSContext, type WSContextInit } from "hono/ws";
import invariant from "invariant";
import { type AnyConn, CONN_STATE_MANAGER_SYMBOL } from "@/actor/conn/mod";
import { isStaticActorDefinition, lookupInRegistry } from "@/actor/definition";
import {
	isStaticActorInstance,
	type AnyStaticActorInstance,
} from "@/actor/instance/mod";
import { KEYS } from "@/actor/instance/keys";
import {
	type PreloadMap,
	compareBytes,
	createPreloadMap,
} from "@/actor/instance/preload-map";
import { deserializeActorKey } from "@/actor/keys";
import type { Encoding } from "@/actor/protocol/serde";
import { type ActorRouter, createActorRouter } from "@/actor/router";
import {
	parseWebSocketProtocols,
	routeWebSocket,
	truncateRawWebSocketPathPrefix,
	type UpgradeWebSocketArgs,
} from "@/actor/router-websocket-endpoints";
import type { Client } from "@/client/client";
import {
	PATH_CONNECT,
	PATH_INSPECTOR_CONNECT,
	PATH_WEBSOCKET_BASE,
	PATH_WEBSOCKET_PREFIX,
} from "@/common/actor-router-consts";
import { getLogger } from "@/common/log";
import { deconstructError } from "@/common/utils";
import {
	buildHibernatableWebSocketAckStateTestResponse,
	type IndexedWebSocketPayload,
	parseHibernatableWebSocketAckStateTestRequest,
	registerRemoteHibernatableWebSocketAckHooks,
	setHibernatableWebSocketAckTestHooks,
	unregisterRemoteHibernatableWebSocketAckHooks,
} from "@/common/websocket-test-hooks";
import type {
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import type { ActorDriver } from "@/actor/driver";
import type { AnyActorInstance } from "@/actor/instance/mod";
import {
	getInitialActorKvState,
	type EngineControlClient,
} from "@/driver-helpers/mod";
import { DynamicActorInstance } from "@/dynamic/instance";
import { DynamicActorIsolateRuntime } from "@/dynamic/isolate-runtime";
import { isDynamicActorDefinition } from "@/dynamic/internal";
import { buildActorNames, type RegistryConfig } from "@/registry/config";
import { getEndpoint } from "@/engine-client/api-utils";
import {
	type LongTimeoutHandle,
	promiseWithResolvers,
	setLongTimeout,
	stringifyError,
	VERSION,
} from "@/utils";
import { getRequireFn } from "@/utils/node";
import { logger } from "./log";

const ENVOY_SSE_PING_INTERVAL = 1000;
const ENVOY_STOP_WAIT_MS = 15_000;
const INITIAL_SLEEP_TIMEOUT_MS = 250;
const REMOTE_ACK_HOOK_QUERY_PARAM = "__rivetkitAckHook";

// Message ack deadline is 30s on the gateway, but we will ack more frequently
// in order to minimize the message buffer size on the gateway and to give
// generous breathing room for the timeout.
//
// See engine/packages/pegboard-gateway/src/shared_state.rs
// (HWS_MESSAGE_ACK_TIMEOUT)
const CONN_MESSAGE_ACK_DEADLINE = 5_000;

// Force saveState when cumulative message size reaches this threshold (0.5 MB)
//
// See engine/packages/pegboard-gateway/src/shared_state.rs
// (HWS_MAX_PENDING_MSGS_SIZE_PER_REQ)
const CONN_BUFFERED_MESSAGE_SIZE_THRESHOLD = 500_000;

interface ActorHandler {
	actor?: AnyActorInstance;
	actorName?: string;
	actorStartPromise?: ReturnType<typeof promiseWithResolvers<void>>;
	actorStartError?: Error;
	alarmTimeout?: LongTimeoutHandle;
	alarmTimestamp?: number;
}

interface HibernatableWebSocketAckState {
	lastSentIndex: number;
	lastAckedIndex: number;
	pendingIndexes: number[];
	ackWaiters: Map<number, Array<() => void>>;
}

export type DriverContext = {};

export class EngineActorDriver implements ActorDriver {
	#config: RegistryConfig;
	#engineClient: EngineControlClient;
	#inlineClient: Client<any>;
	#envoy: EnvoyHandle;
	#actors: Map<string, ActorHandler> = new Map();
	#dynamicRuntimes = new Map<string, DynamicActorIsolateRuntime>();
	#hibernatableWebSocketAcks = new Map<
		string,
		HibernatableWebSocketAckState
	>();
	#hwsMessageIndex = new Map<
		string,
		{
			serverMessageIndex: number;
			bufferedMessageSize: number;
			pendingAckFromMessageIndex: boolean;
			pendingAckFromBufferSize: boolean;
		}
	>();
	#actorRouter: ActorRouter;
	#sqlitePool: SqliteVfsPoolManager;

	#envoyStarted: PromiseWithResolvers<void> = promiseWithResolvers(
		(reason) =>
			logger().warn({
				msg: "unhandled envoy started promise rejection",
				reason,
			}),
	);
	#envoyStopped: PromiseWithResolvers<void> = promiseWithResolvers(
		(reason) =>
			logger().warn({
				msg: "unhandled envoy stopped promise rejection",
				reason,
			}),
	);
	#isEnvoyStopped: boolean = false;
	#isShuttingDown: boolean = false;

	// HACK: Track actor stop intent locally since the envoy protocol doesn't
	// pass the stop reason to onActorStop. This will be fixed when the envoy
	// protocol is updated to send the intent directly (see RVT-5284)
	#actorStopIntent: Map<string, "sleep" | "destroy" | "crash"> = new Map();

	constructor(
		config: RegistryConfig,
		engineClient: EngineControlClient,
		inlineClient: Client<any>,
	) {
		this.#config = config;
		this.#engineClient = engineClient;
		this.#inlineClient = inlineClient;
		this.#sqlitePool = new SqliteVfsPoolManager(config);

		// HACK: Override inspector token (which are likely to be
		// removed later on) with token from x-rivet-token header
		// TODO:
		// if (token && runConfig.inspector && runConfig.inspector.enabled) {
		// 	runConfig.inspector.token = () => token;
		// }

		this.#actorRouter = createActorRouter(
			config,
			this,
			undefined,
			config.test.enabled,
		);

		// Create configuration
		const envoyConfig: EnvoyConfig = {
			version: config.envoy.version,
			endpoint: getEndpoint(config),
			token: config.token,
			namespace: config.namespace,
			poolName: config.envoy.poolName,
			notGlobal: false,
			metadata: {
				rivetkit: { version: VERSION },
			},
			prepopulateActorNames: buildActorNames(config),
			onShutdown: () => {
				this.#envoyStopped.resolve();
				this.#isEnvoyStopped = true;
			},
			fetch: this.#envoyFetch.bind(this),
			websocket: this.#envoyWebSocket.bind(this),
			hibernatableWebSocket: {
				canHibernate: this.#hwsCanHibernate.bind(this),
			},
			onActorStart: this.#envoyOnActorStart.bind(this),
			onActorStop: this.#envoyOnActorStop.bind(this),
			logger: getLogger("envoy-client"),
			debugLatencyMs: process.env._RIVET_DEBUG_LATENCY_MS
				? Number.parseInt(process.env._RIVET_DEBUG_LATENCY_MS, 10)
				: undefined,
		};

		// Create and start envoy
		const { startEnvoySync } = getRequireFn()(
			/* webpackIgnore: true */ "@rivetkit/rivetkit-native/wrapper",
		) as typeof import("@rivetkit/rivetkit-native/wrapper");
		const envoy = startEnvoySync(envoyConfig);

		this.#envoy = envoy;

		envoy.started().then(() => {
			this.#envoyStarted.resolve();
		});

		logger().debug({
			msg: "envoy client started",
			endpoint: config.endpoint,
			namespace: config.namespace,
			poolName: config.envoy.poolName,
		});
	}

	async #discardCrashedActorState(actorId: string) {
		const handler = this.#actors.get(actorId);
		if (!handler) {
			return;
		}

		if (handler.alarmTimeout) {
			handler.alarmTimeout.abort();
			handler.alarmTimeout = undefined;
		}

		if (handler.actor && isStaticActorInstance(handler.actor)) {
			try {
				await handler.actor.debugForceCrash();
			} catch (err) {
				logger().debug({
					msg: "actor crash cleanup errored",
					actorId,
					err: stringifyError(err),
				});
			}
		}

		this.#actors.delete(actorId);
		this.#actorStopIntent.delete(actorId);
	}

	getExtraActorLogParams(): Record<string, string> {
		return { envoyKey: this.#envoy.getEnvoyKey() ?? "-" };
	}

	#hibernatableWebSocketAckKey(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
	): string {
		return `${Buffer.from(gatewayId).toString("hex")}:${Buffer.from(requestId).toString("hex")}`;
	}

	#ensureHibernatableWebSocketAckState(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
	): HibernatableWebSocketAckState {
		const key = this.#hibernatableWebSocketAckKey(gatewayId, requestId);
		let state = this.#hibernatableWebSocketAcks.get(key);
		if (!state) {
			state = {
				lastSentIndex: 0,
				lastAckedIndex: 0,
				pendingIndexes: [],
				ackWaiters: new Map(),
			};
			this.#hibernatableWebSocketAcks.set(key, state);
		}
		return state;
	}

	#deleteHibernatableWebSocketAckState(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
	): void {
		this.#hibernatableWebSocketAcks.delete(
			this.#hibernatableWebSocketAckKey(gatewayId, requestId),
		);
	}

	#recordInboundHibernatableWebSocketMessage(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		rivetMessageIndex: number,
	): void {
		const state = this.#ensureHibernatableWebSocketAckState(
			gatewayId,
			requestId,
		);
		state.lastSentIndex = Math.max(state.lastSentIndex, rivetMessageIndex);
		if (!state.pendingIndexes.includes(rivetMessageIndex)) {
			state.pendingIndexes.push(rivetMessageIndex);
			state.pendingIndexes.sort((a, b) => a - b);
		}
	}

	#recordAckedHibernatableWebSocketMessage(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		serverMessageIndex: number,
	): void {
		const state = this.#ensureHibernatableWebSocketAckState(
			gatewayId,
			requestId,
		);
		state.lastAckedIndex = Math.max(
			state.lastAckedIndex,
			serverMessageIndex,
		);
		state.pendingIndexes = state.pendingIndexes.filter(
			(index) => index > serverMessageIndex,
		);
		for (const [index, waiters] of state.ackWaiters) {
			if (index > serverMessageIndex) {
				continue;
			}
			state.ackWaiters.delete(index);
			for (const resolve of waiters) {
				resolve();
			}
		}
	}

	#registerHibernatableWebSocketAckTestHooks(
		websocket: UniversalWebSocket,
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		remoteHookToken?: string,
	): void {
		setHibernatableWebSocketAckTestHooks(
			websocket,
			{
				getState: () => {
					const state = this.#ensureHibernatableWebSocketAckState(
						gatewayId,
						requestId,
					);
					return {
						lastSentIndex: state.lastSentIndex,
						lastAckedIndex: state.lastAckedIndex,
						pendingIndexes: [...state.pendingIndexes],
					};
				},
				waitForAck: async (serverMessageIndex) => {
					const state = this.#ensureHibernatableWebSocketAckState(
						gatewayId,
						requestId,
					);
					if (state.lastAckedIndex >= serverMessageIndex) {
						return;
					}
					await new Promise<void>((resolve) => {
						const waiters =
							state.ackWaiters.get(serverMessageIndex) ?? [];
						waiters.push(resolve);
						state.ackWaiters.set(serverMessageIndex, waiters);
					});
				},
			},
			this.#config.test.enabled,
		);
		registerRemoteHibernatableWebSocketAckHooks(
			remoteHookToken ?? "",
			{
				getState: () => {
					const state = this.#ensureHibernatableWebSocketAckState(
						gatewayId,
						requestId,
					);
					return {
						lastSentIndex: state.lastSentIndex,
						lastAckedIndex: state.lastAckedIndex,
						pendingIndexes: [...state.pendingIndexes],
					};
				},
				waitForAck: async (serverMessageIndex) => {
					const state = this.#ensureHibernatableWebSocketAckState(
						gatewayId,
						requestId,
					);
					if (state.lastAckedIndex >= serverMessageIndex) {
						return;
					}
					await new Promise<void>((resolve) => {
						const waiters =
							state.ackWaiters.get(serverMessageIndex) ?? [];
						waiters.push(resolve);
						state.ackWaiters.set(serverMessageIndex, waiters);
					});
				},
			},
			this.#config.test.enabled && Boolean(remoteHookToken),
		);
	}

	#maybeRespondToHibernatableAckStateProbe(
		websocket: UniversalWebSocket,
		data: IndexedWebSocketPayload,
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
	): boolean {
		if (
			!parseHibernatableWebSocketAckStateTestRequest(
				data,
				this.#config.test.enabled,
			)
		) {
			return false;
		}

		const state = this.#ensureHibernatableWebSocketAckState(
			gatewayId,
			requestId,
		);
		const response = buildHibernatableWebSocketAckStateTestResponse(
			{
				lastSentIndex: state.lastSentIndex,
				lastAckedIndex: state.lastAckedIndex,
				pendingIndexes: [...state.pendingIndexes],
			},
			this.#config.test.enabled,
		);
		invariant(response, "missing hibernatable websocket ack test response");
		websocket.send(response);
		return true;
	}

	async #loadActorHandler(actorId: string): Promise<ActorHandler> {
		// Check if actor is already loaded
		const handler = this.#actors.get(actorId);
		if (!handler)
			throw new Error(`Actor handler does not exist ${actorId}`);
		if (handler.actorStartPromise) await handler.actorStartPromise.promise;
		if (handler.actorStartError) throw handler.actorStartError;
		if (!handler.actor) throw new Error("Actor should be loaded");
		return handler;
	}

	getContext(actorId: string): DriverContext {
		return {};
	}

	cancelAlarm(actorId: string): void {
		const handler = this.#actors.get(actorId);
		if (handler?.alarmTimeout) {
			handler.alarmTimeout.abort();
			handler.alarmTimeout = undefined;
		}
	}

	#isDynamicActor(actorId: string): boolean {
		return this.#dynamicRuntimes.has(actorId);
	}

	#requireDynamicRuntime(actorId: string): DynamicActorIsolateRuntime {
		const runtime = this.#dynamicRuntimes.get(actorId);
		if (!runtime) {
			throw new Error(
				`dynamic runtime is not loaded for actor ${actorId}`,
			);
		}
		return runtime;
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		const handler = this.#actors.get(actor.id);
		if (!handler) {
			logger().warn({
				msg: "no handler for actor to set alarm",
			});

			return;
		}

		// Clear prev timeout
		if (handler.alarmTimeout && handler.alarmTimestamp === timestamp) {
			return;
		}

		if (handler.alarmTimeout) {
			handler.alarmTimeout.abort();
			handler.alarmTimeout = undefined;
		}

		// Set alarm
		const delay = Math.max(0, timestamp - Date.now());
		handler.alarmTimestamp = timestamp;
		handler.alarmTimeout = setLongTimeout(() => {
			void (async () => {
				const currentHandler = this.#actors.get(actor.id);
				if (!currentHandler) {
					logger().debug({
						msg: "alarm fired without loaded actor",
						actorId: actor.id,
					});
					return;
				}

				if (currentHandler.actorStartPromise) {
					try {
						await currentHandler.actorStartPromise.promise;
					} catch (error) {
						logger().debug({
							msg: "alarm skipped after actor failed to start",
							actorId: actor.id,
							error: stringifyError(error),
						});
						return;
					}
				}

				const alarmActor = this.#actors.get(actor.id)?.actor;
				if (!alarmActor || alarmActor.isStopping) {
					logger().debug({
						msg: "alarm fired without ready actor",
						actorId: actor.id,
					});
					return;
				}

				await alarmActor.onAlarm();
			})().catch((error) => {
				logger().error({
					msg: "actor alarm failed",
					actorId: actor.id,
					error: stringifyError(error),
				});
			});
			handler.alarmTimeout = undefined;
			handler.alarmTimestamp = undefined;
		}, delay);

		// TODO: This call may not be needed on ActorInstance.start, but it does help ensure that the local state is synced with the alarm state
		// Set alarm on Rivet
		//
		// This does not call an "alarm" event like Durable Objects.
		// Instead, it just wakes the actor on the alarm (if not
		// already awake).
		//
		// onAlarm is automatically called on `ActorInstance.start` when waking
		// again.
		this.#envoy.setAlarm(actor.id, timestamp);
	}

	// No database overrides - will use KV-backed implementation from rivetkit/db

	getInitialSleepTimeoutMs(
		_actor: AnyActorInstance,
		defaultTimeoutMs: number,
	): number {
		return Math.max(defaultTimeoutMs, INITIAL_SLEEP_TIMEOUT_MS);
	}

	getNativeSqliteConfig() {
		return {
			endpoint: getEndpoint(this.#config),
			token: this.#config.token,
			namespace: this.#config.namespace,
		};
	}

	getNativeDatabaseProvider() {
		// Try to load the native package. If available, return a provider
		// that opens databases from the live envoy handle.
		try {
			const requireFn = getRequireFn();

			const nativeMod = requireFn(
				/* webpackIgnore: true */ "@rivetkit/rivetkit-native/wrapper",
			);
			if (!nativeMod?.openRawDatabaseFromEnvoy) return undefined;

			const envoy = this.#envoy;
			return {
				open: async (actorId: string) => {
					return await nativeMod.openRawDatabaseFromEnvoy(
						envoy,
						actorId,
					);
				},
			};
		} catch {
			return undefined;
		}
	}

	// MARK: - Batch KV operations
	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		await this.#envoy.kvPut(actorId, entries);
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		return await this.#envoy.kvGet(actorId, keys);
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		await this.#envoy.kvDelete(actorId, keys);
	}

	async kvDeleteRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
	): Promise<void> {
		await this.#envoy.kvDeleteRange(actorId, start, end);
	}

	async kvList(actorId: string): Promise<Uint8Array[]> {
		const entries = await this.#envoy.kvListPrefix(
			actorId,
			new Uint8Array(),
		);
		const keys = entries.map(([key]) => key);
		return keys;
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
		options?: {
			reverse?: boolean;
			limit?: number;
		},
	): Promise<[Uint8Array, Uint8Array][]> {
		const result = await this.#envoy.kvListPrefix(
			actorId,
			prefix,
			options,
		);
		logger().info({
			msg: "kvListPrefix called",
			actorId,
			prefixStr: new TextDecoder().decode(prefix),
			entriesCount: result.length,
			keys: result.map(([key]: [Uint8Array, ...unknown[]]) => new TextDecoder().decode(key)),
		});
		return result;
	}

	async kvListRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
		options?: {
			reverse?: boolean;
			limit?: number;
		},
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#envoy.kvListRange(
			actorId,
			start,
			end,
			true,
			options,
		);
	}

	ackHibernatableWebSocketMessage(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		serverMessageIndex: number,
	): void {
		this.#recordAckedHibernatableWebSocketMessage(
			gatewayId,
			requestId,
			serverMessageIndex,
		);
		this.#envoy.sendHibernatableWebSocketMessageAck(
			gatewayId,
			requestId,
			serverMessageIndex,
		);
	}

	/** Creates a SQLite VFS instance for creating KV-backed databases */
	async createSqliteVfs(actorId: string): Promise<ISqliteVfs> {
		return await this.#sqlitePool.acquire(actorId);
	}

	// MARK: - Actor Lifecycle
	async loadActor(actorId: string): Promise<AnyActorInstance> {
		const handler = await this.#loadActorHandler(actorId);
		if (!handler.actor) throw new Error(`Actor ${actorId} failed to load`);
		return handler.actor;
	}

	startSleep(actorId: string) {
		// HACK: Track intent for onActorStop (see RVT-5284)
		this.#actorStopIntent.set(actorId, "sleep");
		this.#envoy.sleepActor(actorId);
	}

	startDestroy(actorId: string) {
		// HACK: Track intent for onActorStop (see RVT-5284)
		this.#actorStopIntent.set(actorId, "destroy");
		this.#envoy.destroyActor(actorId);
	}

	async hardCrashActor(actorId: string): Promise<void> {
		const handler = this.#actors.get(actorId);
		if (!handler) {
			return;
		}

		if (handler.actorStartPromise) {
			await handler.actorStartPromise.promise.catch(() => undefined);
		}

		logger().info({
			msg: "simulating hard crash for actor",
			actorId,
		});

		await this.#discardCrashedActorState(actorId);
		this.#actorStopIntent.set(actorId, "crash");
		this.#envoy.stopActor(actorId, undefined, "simulated hard crash");
	}

	async shutdown(immediate: boolean): Promise<void> {
		if (this.#isShuttingDown) {
			return;
		}
		this.#isShuttingDown = true;

		logger().info({ msg: "stopping engine actor driver", immediate });
		if (!immediate) {
			// Put actors through the normal sleep intent path before draining the
			// runner. This ensures Pegboard marks the actor workflow as sleeping
			// and preserves wakeability across runner handoff.
			logger().debug({
				msg: "sending sleep intent to actors before runner drain",
				actorCount: this.#actors.size,
			});
			for (const actorId of this.#actors.keys()) {
				this.startSleep(actorId);
			}

			const actorSleepDeadline = Date.now() + ENVOY_STOP_WAIT_MS;
			while (this.#actors.size > 0 && Date.now() < actorSleepDeadline) {
				await new Promise((resolve) => setTimeout(resolve, 50));
			}

			if (this.#actors.size > 0) {
				logger().warn({
					msg: "timed out waiting for actors to stop before envoy drain",
					remainingActors: this.#actors.size,
					waitMs: ENVOY_STOP_WAIT_MS,
				});
			} else {
				logger().debug({
					msg: "all actors stopped before envoy drain",
				});
			}
		}

		await this.#sqlitePool.shutdown();

		try {
			await this.#envoy.shutdown(immediate);
		} catch (error) {
			const message =
				error instanceof Error ? error.message : String(error);
			if (
				message.includes("WebSocket connection closed during shutdown")
			) {
				logger().debug({
					msg: "ignoring shutdown websocket close race",
					error: message,
				});
			} else {
				throw error;
			}
		}

		const stopped = await Promise.race([
			this.#envoyStopped.promise.then(() => true),
			new Promise<false>((resolve) =>
				setTimeout(() => resolve(false), ENVOY_STOP_WAIT_MS),
			),
		]);
		if (!stopped) {
			logger().warn({
				msg: "timed out waiting for envoy shutdown",
				waitMs: ENVOY_STOP_WAIT_MS,
			});
		}

		this.#dynamicRuntimes.clear();
	}

	async waitForReady(): Promise<void> {
		await this.#envoy.started();
	}

	async serverlessHandleStart(c: HonoContext): Promise<Response> {
		let payload = await c.req.arrayBuffer();

		return streamSSE(c, async (stream) => {
			// NOTE: onAbort does not work reliably
			stream.onAbort(() => { });
			c.req.raw.signal.addEventListener("abort", () => {
				logger().debug("SSE aborted");
			});

			await this.#envoyStarted.promise;

			if (this.#isShuttingDown) {
				logger().debug({
					msg: "ignoring serverless start because driver is shutting down",
				});
				return;
			}

			await this.#envoy.startServerlessActor(payload);

			// Send ping every second to keep the connection alive
			while (true) {
				if (this.#isEnvoyStopped) {
					logger().debug({
						msg: "envoy is stopped",
					});
					break;
				}

				if (stream.closed || stream.aborted) {
					logger().debug({
						msg: "envoy sse stream closed",
						closed: stream.closed,
						aborted: stream.aborted,
					});
					break;
				}

				await stream.writeSSE({ event: "ping", data: "" });
				await stream.sleep(ENVOY_SSE_PING_INTERVAL);
			}
		});
	}

	#buildStartupPreloadMap(
		preloadedKv: protocol.PreloadedKv | null,
		persistDataOverride?: Uint8Array,
	): { preloadMap: PreloadMap | undefined; entries: number } {
		if (preloadedKv == null) {
			return { preloadMap: undefined, entries: 0 };
		}

		const entries: [Uint8Array, Uint8Array][] = preloadedKv.entries.map(
			(entry) => [new Uint8Array(entry.key), new Uint8Array(entry.value)],
		);

		if (persistDataOverride) {
			let replaced = false;
			for (const entry of entries) {
				if (compareBytes(entry[0], KEYS.PERSIST_DATA) === 0) {
					entry[1] = persistDataOverride;
					replaced = true;
					break;
				}
			}

			if (!replaced) {
				entries.push([KEYS.PERSIST_DATA, persistDataOverride]);
			}
		}

		entries.sort((a, b) => compareBytes(a[0], b[0]));

		const requestedGetKeys = preloadedKv.requestedGetKeys
			.map((key) => new Uint8Array(key))
			.sort(compareBytes);
		const requestedPrefixes = preloadedKv.requestedPrefixes
			.map((prefix) => new Uint8Array(prefix))
			.sort(compareBytes);

		return {
			preloadMap: createPreloadMap(
				entries,
				requestedGetKeys,
				requestedPrefixes,
			),
			entries: entries.length,
		};
	}

	async #envoyOnActorStart(
		_envoy: EnvoyHandle,
		actorId: string,
		generation: number,
		actorConfig: protocol.ActorConfig,
		preloadedKv: protocol.PreloadedKv | null,
	): Promise<void> {
		if (this.#isShuttingDown) {
			logger().debug({
				msg: "rejecting actor start because driver is shutting down",
				actorId,
				name: actorConfig.name,
				generation,
			});
			throw new Error("engine actor driver is shutting down");
		}

		logger().debug({
			msg: "engine actor starting",
			actorId,
			name: actorConfig.name,
			key: actorConfig.key,
			generation,
		});

		// Deserialize input
		let input: any;
		if (actorConfig.input) {
			input = cbor.decode(new Uint8Array(actorConfig.input));
		}

		// Get or create handler
		let handler = this.#actors.get(actorId);
		if (!handler) {
			// IMPORTANT: We must set the handler in the map synchronously before doing any
			// async operations to avoid race conditions where multiple calls might try to
			// create the same handler simultaneously.
			handler = {
				actorStartPromise: promiseWithResolvers((reason) =>
					logger().warn({
						msg: "unhandled actor start promise rejection",
						reason,
					}),
				),
			};
			this.#actors.set(actorId, handler);
		}
		handler.actorStartError = undefined;

		const name = actorConfig.name as string;
		invariant(actorConfig.key, "actor should have a key");
		const key = deserializeActorKey(actorConfig.key);
		handler.actorName = name;

		try {
			let preloadMap: PreloadMap | undefined;
			let persistDataBuffer: Uint8Array | null | undefined;
			let checkPersistDataMs = 0;
			let initNewActorMs = 0;
			let preloadKvMs = 0;
			let preloadKvEntries = 0;
			let driverKvRoundTrips = 0;

			if (preloadedKv) {
				const preloadStart = performance.now();
				const preloaded = this.#buildStartupPreloadMap(preloadedKv);
				preloadMap = preloaded.preloadMap;
				preloadKvEntries = preloaded.entries;
				preloadKvMs = performance.now() - preloadStart;
				persistDataBuffer = preloadMap?.get(KEYS.PERSIST_DATA)?.value;
				logger().debug({
					msg: "received startup kv preload from start command",
					actorId,
					entries: preloadKvEntries,
					durationMs: preloadKvMs,
				});
			}

			if (persistDataBuffer === undefined) {
				const checkStart = performance.now();
				const [persistData] = await this.#envoy.kvGet(actorId, [
					KEYS.PERSIST_DATA,
				]);
				persistDataBuffer = persistData;
				checkPersistDataMs = performance.now() - checkStart;
				driverKvRoundTrips++;
			}

			if (persistDataBuffer === null) {
				const initStart = performance.now();
				const initialKvState = getInitialActorKvState(input);
				const persistData = initialKvState[0]?.[1];
				await this.#envoy.kvPut(actorId, initialKvState);
				initNewActorMs = performance.now() - initStart;
				driverKvRoundTrips++;
				if (preloadedKv && persistData) {
					const preloadStart = performance.now();
					const preloaded = this.#buildStartupPreloadMap(
						preloadedKv,
						persistData,
					);
					preloadMap = preloaded.preloadMap;
					preloadKvEntries = preloaded.entries;
					preloadKvMs += performance.now() - preloadStart;
				}
				logger().debug({
					msg: "initialized persist data for new actor",
					actorId,
					durationMs: initNewActorMs,
				});
			}

			// Create actor instance
			const definition = lookupInRegistry(this.#config, actorConfig.name);
			if (isDynamicActorDefinition(definition)) {
				let runtime = this.#dynamicRuntimes.get(actorId);
				if (!runtime) {
					runtime = new DynamicActorIsolateRuntime({
						actorId,
						actorName: name,
						actorKey: key,
						input,
						region: "unknown",
						loader: definition.loader,
						actorDriver: this,
						inlineClient: this.#inlineClient,
						test: this.#config.test,
					});
					await runtime.start();
					this.#dynamicRuntimes.set(actorId, runtime);
				}

				const dynamicActor = new DynamicActorInstance(actorId, runtime);
				handler.actor = dynamicActor;

				handler.actorStartError = undefined;
				handler.actorStartPromise?.resolve();
				handler.actorStartPromise = undefined;

				const rawMetaEntries =
					await dynamicActor.getHibernatingWebSockets();
				const metaEntries = rawMetaEntries.map((entry) => ({
					gatewayId: entry.gatewayId,
					requestId: entry.requestId,
					rivetMessageIndex: entry.serverMessageIndex,
					envoyMessageIndex: entry.clientMessageIndex,
					path: entry.path,
					headers: entry.headers,
				}));
				await this.#envoy.restoreHibernatingRequests(
					actorId,
					metaEntries,
				);
			} else if (isStaticActorDefinition(definition)) {
				const instantiateStart = performance.now();
				const staticActor =
					(await definition.instantiate()) as AnyStaticActorInstance;
				const instantiateMs = performance.now() - instantiateStart;
				handler.actor = staticActor;

				// Record driver-level startup metrics on the actor.
				staticActor.metrics.startup.checkPersistDataMs =
					checkPersistDataMs;
				staticActor.metrics.startup.initNewActorMs = initNewActorMs;
				staticActor.metrics.startup.preloadKvMs = preloadKvMs;
				staticActor.metrics.startup.preloadKvEntries = preloadKvEntries;
				staticActor.metrics.startup.instantiateMs = instantiateMs;
				staticActor.metrics.startup.kvRoundTrips = driverKvRoundTrips;

				// Apply protocol limits as per-instance overrides without mutating the shared definition
				const protocolMetadata = this.#envoy.getProtocolMetadata();
				if (protocolMetadata) {
					const stopThresholdMax = Math.max(
						Number(protocolMetadata.actorStopThreshold) - 1000,
						0,
					);
					staticActor.overrides.onSleepTimeout = stopThresholdMax;
					staticActor.overrides.onDestroyTimeout = stopThresholdMax;

					if (protocolMetadata.serverlessDrainGracePeriod) {
						const drainMax = Math.max(
							Number(
								protocolMetadata.serverlessDrainGracePeriod,
							) - 1000,
							0,
						);
						staticActor.overrides.runStopTimeout = drainMax;
						staticActor.overrides.waitUntilTimeout = drainMax;
						staticActor.overrides.sleepGracePeriod =
							stopThresholdMax + drainMax;
					}
				}

				// Start actor
				await staticActor.start(
					this,
					this.#inlineClient,
					actorId,
					name,
					key,
					"unknown", // TODO: Add regions
					preloadMap,
				);
			} else {
				throw new Error(
					`actor definition for ${actorConfig.name} is not instantiable`,
				);
			}

			logger().debug({ msg: "engine actor started", actorId, name, key });
		} catch (innerError) {
			const dynamicRuntime = this.#dynamicRuntimes.get(actorId);
			if (dynamicRuntime) {
				try {
					await dynamicRuntime.dispose();
				} catch (disposeError) {
					logger().debug({
						msg: "failed to dispose dynamic runtime after actor start failure",
						actorId,
						err: stringifyError(disposeError),
					});
				}
				this.#dynamicRuntimes.delete(actorId);
			}
			const error =
				innerError instanceof Error
					? new Error(
						`Failed to start actor ${actorId}: ${innerError.message}`,
						{ cause: innerError },
					)
					: new Error(
						`Failed to start actor ${actorId}: ${String(innerError)}`,
					);
			handler.actor = undefined;
			handler.actorStartError = error;
			handler.actorStartPromise?.reject(error);
			handler.actorStartPromise = undefined;
			logger().error({
				msg: "engine actor failed to start",
				actorId,
				name,
				key,
				err: stringifyError(error),
			});

			try {
				this.#envoy.stopActor(actorId, undefined);
			} catch (stopError) {
				logger().debug({
					msg: "failed to stop actor after start failure",
					actorId,
					err: stringifyError(stopError),
				});
			}
		}
	}

	async #envoyOnActorStop(
		_envoyHandle: EnvoyHandle,
		actorId: string,
		generation: number,
		_reason: protocol.StopActorReason,
	): Promise<void> {
		logger().debug({ msg: "engine actor stopping", actorId, generation });

		// HACK: Retrieve the stop intent we tracked locally (see RVT-5284)
		// Default to "sleep" if no intent was recorded (e.g., if the runner
		// initiated the stop)
		//
		// TODO: This will not work if the actor is destroyed from the API
		// correctly. Currently, it will use the sleep intent, but it's
		// actually a destroy intent.
		const reason = this.#actorStopIntent.get(actorId) ?? "sleep";
		this.#actorStopIntent.delete(actorId);

		const handler = this.#actors.get(actorId);
		if (!handler) {
			logger().debug({
				msg: "no engine actor handler to stop",
				actorId,
				reason,
			});
			return;
		}

		if (handler.actorStartPromise) {
			try {
				logger().debug({
					msg: "engine actor stopping before it started, waiting",
					actorId,
					generation,
				});
				await handler.actorStartPromise.promise;
			} catch (err) {
				// Start failed, but we still want to clean up the handler
				logger().debug({
					msg: "actor start failed during stop, cleaning up handler",
					actorId,
					err: stringifyError(err),
				});
			}
		}

		if (handler.actor) {
			try {
				if (reason === "crash" && isStaticActorInstance(handler.actor)) {
					await handler.actor.debugForceCrash();
				} else if (reason !== "crash") {
					await handler.actor.onStop(reason);
				}
			} catch (err) {
				logger().error({
					msg: "error in onStop, proceeding with removing actor",
					err: stringifyError(err),
				});
			}
		}
		this.#dynamicRuntimes.delete(actorId);

		if (handler.alarmTimeout) {
			handler.alarmTimeout.abort();
			handler.alarmTimeout = undefined;
		}

		this.#actors.delete(actorId);

		logger().debug({ msg: "engine actor stopped", actorId, reason });
	}

	// MARK: - Envoy Networking
	async #envoyFetch(
		_envoy: EnvoyHandle,
		actorId: string,
		_gatewayIdBuf: ArrayBuffer,
		_requestIdBuf: ArrayBuffer,
		request: Request,
	): Promise<Response> {
		logger().debug({
			msg: "envoy fetch",
			actorId,
			url: request.url,
			method: request.method,
		});
		const overlayResponse = this.#routeOverlayRequest(actorId, request);
		if (overlayResponse) {
			return overlayResponse;
		}

		if (this.#isDynamicActor(actorId)) {
			return await this.#requireDynamicRuntime(actorId).fetch(request);
		}
		return await this.#actorRouter.fetch(request, { actorId });
	}

	#routeOverlayRequest(
		actorId: string,
		request: Request,
	): Response | null {
		const url = new URL(request.url);
		switch (`${request.method} ${url.pathname}`) {
			case "PUT /dynamic/reload":
				return this.#handleDynamicReloadOverlay(actorId);
			default:
				return null;
		}
	}

	#handleDynamicReloadOverlay(actorId: string): Response {
		if (!this.#isDynamicActor(actorId)) {
			return new Response("not a dynamic actor", { status: 404 });
		}
		this.startSleep(actorId);
		return new Response(null, { status: 200 });
	}

	async #envoyWebSocket(
		_envoy: EnvoyHandle,
		actorId: string,
		websocketRaw: any,
		gatewayIdBuf: ArrayBuffer,
		requestIdBuf: ArrayBuffer,
		request: Request,
		requestPath: string,
		requestHeaders: Record<string, string>,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
	): Promise<void> {
		const websocket = websocketRaw as UniversalWebSocket;

		// Parse configuration from Sec-WebSocket-Protocol header (optional for path-based routing)
		const protocols = request.headers.get("sec-websocket-protocol");
		const { encoding, connParams, ackHookToken } =
			parseWebSocketProtocols(protocols);
		const remoteAckHookToken =
			ackHookToken ??
			new URL(request.url).searchParams.get(
				REMOTE_ACK_HOOK_QUERY_PARAM,
			) ??
			undefined;

		if (this.#isDynamicActor(actorId)) {
			await this.#runnerDynamicWebSocket(
				actorId,
				websocket,
				gatewayIdBuf,
				requestIdBuf,
				requestPath,
				requestHeaders,
				encoding,
				connParams,
				isHibernatable,
				isRestoringHibernatable,
			);
			return;
		}

		// Fetch WS handler
		//
		// We store the promise since we need to add WebSocket event listeners immediately that will wait for the promise to resolve
		let wsHandler: UpgradeWebSocketArgs;
		try {
			wsHandler = await routeWebSocket(
				request,
				requestPath,
				requestHeaders,
				this.#config,
				this,
				actorId,
				encoding,
				connParams,
				gatewayIdBuf,
				requestIdBuf,
				isHibernatable,
				isRestoringHibernatable,
			);
		} catch (err) {
			logger().error({ msg: "building websocket handlers errored", err });
			websocketRaw.close(1011, "ws.route_error");
			return;
		}

		// Connect the Hono WS hook to the adapter
		//
		// We need to assign to `raw` in order for WSContext to expose it on
		// `ws.raw`
		(websocket as WSContextInit).raw = websocket;
		const wsContext = new WSContext(websocket);

		// Get connection and actor from wsHandler (may be undefined for inspector endpoint)
		const conn = wsHandler.conn;
		const actor = wsHandler.actor;
		const connStateManager = conn?.[CONN_STATE_MANAGER_SYMBOL];

		// Bind event listeners to Hono WebSocket handlers
		//
		// We update the HWS data after calling handlers in order to ensure
		// that the handler ran successfully. By doing this, we ensure at least
		// once delivery of events to the event handlers.

		if (isHibernatable) {
			this.#registerHibernatableWebSocketAckTestHooks(
				websocket,
				gatewayIdBuf,
				requestIdBuf,
				remoteAckHookToken,
			);
		}

		if (isRestoringHibernatable) {
			wsHandler.onRestore?.(wsContext);
		}

		const isRawWebSocketPath =
			requestPath === PATH_WEBSOCKET_BASE ||
			requestPath.startsWith(PATH_WEBSOCKET_PREFIX);
		const handleMessageEvent = (event: RivetMessageEvent) => {
			if (
				isHibernatable &&
				this.#maybeRespondToHibernatableAckStateProbe(
					websocket,
					event.data,
					gatewayIdBuf,
					requestIdBuf,
				)
			) {
				return;
			}

			if (actor?.isStopping) {
				logger().debug({
					msg: "ignoring ws message, actor is stopping",
					connId: conn?.id,
					actorId: actor?.id,
					messageIndex: event.rivetMessageIndex,
				});
				return;
			}

			const run = async () => {
				// Process message
				if (isHibernatable && typeof event.rivetMessageIndex === "number") {
					this.#recordInboundHibernatableWebSocketMessage(
						gatewayIdBuf,
						requestIdBuf,
						event.rivetMessageIndex,
					);
				}
				wsHandler.onMessage(event, wsContext);

				// Runtime-owned hibernatable websocket bookkeeping lives on the
				// actor instance so static and dynamic paths share the same logic.
				if (conn && actor && isStaticActorInstance(actor)) {
					actor.handleInboundHibernatableWebSocketMessage(
						conn,
						event.data,
						event.rivetMessageIndex,
					);
				}
			};

			if (isRawWebSocketPath && actor) {
				void actor.internalKeepAwake(run);
			} else {
				void run();
			}
		};
		const attachMessageListener = () => {
			websocket.addEventListener("message", handleMessageEvent);
		};
		let postOpenListenersAttached = false;
		const attachPostOpenListeners = () => {
			if (postOpenListenersAttached) {
				return;
			}
			postOpenListenersAttached = true;

			if (!isRawWebSocketPath) {
				attachMessageListener();
			}

			websocket.addEventListener("close", (event) => {
				if (isRawWebSocketPath && actor) {
					void actor.internalKeepAwake(async () => {
						await Promise.resolve();
						wsHandler.onClose(event, wsContext);
					});
				} else {
					wsHandler.onClose(event, wsContext);
				}
				if (isHibernatable) {
					this.#deleteHibernatableWebSocketAckState(
						gatewayIdBuf,
						requestIdBuf,
					);
					unregisterRemoteHibernatableWebSocketAckHooks(
						remoteAckHookToken,
						this.#config.test.enabled,
					);
				}
			});

			websocket.addEventListener("error", (event) => {
				wsHandler.onError(event, wsContext);
			});
		};

		websocket.addEventListener("open", (event) => {
			if (isRawWebSocketPath) {
				attachMessageListener();
			}

			wsHandler.onOpen(event, wsContext);

			attachPostOpenListeners();
		});

		if (!isRawWebSocketPath) {
			attachPostOpenListeners();
		}
	}

	async #runnerDynamicWebSocket(
		actorId: string,
		websocket: UniversalWebSocket,
		gatewayIdBuf: ArrayBuffer,
		requestIdBuf: ArrayBuffer,
		requestPath: string,
		requestHeaders: Record<string, string>,
		encoding: Encoding,
		connParams: unknown,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
	): Promise<void> {
		let runtime: DynamicActorIsolateRuntime;
		const remoteAckHookToken =
			parseWebSocketProtocols(
				requestHeaders["sec-websocket-protocol"] ?? undefined,
			).ackHookToken ??
			new URL(`http://actor${requestPath}`).searchParams.get(
				REMOTE_ACK_HOOK_QUERY_PARAM,
			) ??
			undefined;
		try {
			runtime = this.#requireDynamicRuntime(actorId);
		} catch (error) {
			logger().error({
				msg: "dynamic runtime missing for websocket",
				actorId,
				error: stringifyError(error),
			});
			websocket.close(1011, "dynamic.runtime_missing");
			return;
		}

		let proxyToActorWs: UniversalWebSocket;
		try {
			proxyToActorWs = await runtime.openWebSocket(
				requestPath,
				encoding,
				connParams,
				{
					headers: requestHeaders,
					gatewayId: gatewayIdBuf,
					requestId: requestIdBuf,
					isHibernatable,
					isRestoringHibernatable,
				},
			);
		} catch (error) {
			const { group, code } = deconstructError(
				error,
				logger(),
				{},
				false,
			);
			logger().error({
				msg: "failed to open dynamic websocket",
				actorId,
				error: stringifyError(error),
			});
			websocket.close(1011, `${group}.${code}`);
			return;
		}

		if (isHibernatable) {
			this.#registerHibernatableWebSocketAckTestHooks(
				websocket,
				gatewayIdBuf,
				requestIdBuf,
				remoteAckHookToken,
			);
		}

		proxyToActorWs.addEventListener(
			"message",
			(event: RivetMessageEvent) => {
				if (websocket.readyState !== websocket.OPEN) {
					return;
				}
				websocket.send(event.data as any);
			},
		);

		proxyToActorWs.addEventListener("close", (event) => {
			if (isHibernatable && event.reason === "dynamic.runtime.disposed") {
				logger().debug({
					msg: "ignoring dynamic runtime dispose close for hibernatable websocket",
					actorId,
					code: event.code,
					reason: event.reason,
				});
				return;
			}
			if (websocket.readyState !== websocket.CLOSED) {
				websocket.close(event.code, event.reason);
			}
		});

		proxyToActorWs.addEventListener("error", (_event) => {
			if (websocket.readyState !== websocket.CLOSED) {
				websocket.close(1011, "dynamic.websocket_error");
			}
		});

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			if (
				isHibernatable &&
				this.#maybeRespondToHibernatableAckStateProbe(
					websocket,
					event.data,
					gatewayIdBuf,
					requestIdBuf,
				)
			) {
				return;
			}

			const actorHandler = this.#actors.get(actorId);
			if (actorHandler?.actor?.isStopping) {
				return;
			}
			if (isHibernatable && typeof event.rivetMessageIndex === "number") {
				this.#recordInboundHibernatableWebSocketMessage(
					gatewayIdBuf,
					requestIdBuf,
					event.rivetMessageIndex,
				);
			}
			void runtime
				.forwardIncomingWebSocketMessage(
					proxyToActorWs,
					event.data as any,
					event.rivetMessageIndex,
				)
				.catch((error) => {
					logger().error({
						msg: "failed forwarding websocket message to dynamic actor",
						actorId,
						error: stringifyError(error),
					});
					websocket.close(1011, "dynamic.websocket_forward_failed");
				});
		});

		websocket.addEventListener("close", (event) => {
			if (isHibernatable) {
				this.#deleteHibernatableWebSocketAckState(
					gatewayIdBuf,
					requestIdBuf,
				);
				unregisterRemoteHibernatableWebSocketAckHooks(
					remoteAckHookToken,
					this.#config.test.enabled,
				);
			}
			if (proxyToActorWs.readyState !== proxyToActorWs.CLOSED) {
				proxyToActorWs.close(event.code, event.reason);
			}
		});

		websocket.addEventListener("error", () => {
			if (proxyToActorWs.readyState !== proxyToActorWs.CLOSED) {
				proxyToActorWs.close(1011, "dynamic.gateway_error");
			}
		});
	}

	// MARK: - Hibernating WebSockets
	#hwsCanHibernate(
		actorId: string,
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		request: Request,
	): boolean {
		const url = new URL(request.url);
		const path = url.pathname;

		// Resolve actor name from either the envoy's actor view or the local
		// handler. WebSocket opens can race with actor startup, so the local
		// handler may know the actor name slightly earlier than the envoy.
		const actorInstance = this.#envoy.getActor(actorId);
		const handler = this.#actors.get(actorId);
		const actorName =
			actorInstance &&
				"config" in actorInstance &&
				actorInstance.config &&
				typeof actorInstance.config === "object" &&
				"name" in actorInstance.config &&
				typeof actorInstance.config.name === "string"
				? actorInstance.config.name
				: handler?.actorName;
		if (!actorName) {
			logger().warn({
				msg: "actor name unavailable in #hwsCanHibernate",
				actorId,
			});
			return false;
		}

		// Determine configuration for new WS
		logger().debug({
			msg: "no existing hibernatable websocket found",
			gatewayId: Buffer.from(gatewayId).toString("hex"),
			requestId: Buffer.from(requestId).toString("hex"),
		});
		if (path === PATH_CONNECT) {
			return true;
		} else if (
			path === PATH_WEBSOCKET_BASE ||
			path.startsWith(PATH_WEBSOCKET_PREFIX)
		) {
			// Find actor config
			// Hibernation capability is a definition-level property, so the
			// envoy can decide it before the actor has fully started.
			const definition = lookupInRegistry(this.#config, actorName);

			// Check if can hibernate
			const canHibernateWebSocket =
				definition.config.options?.canHibernateWebSocket;
			if (canHibernateWebSocket === true) {
				return true;
			} else if (typeof canHibernateWebSocket === "function") {
				try {
					// Truncate the path to match the behavior on onRawWebSocket
					const newPath = truncateRawWebSocketPathPrefix(
						url.pathname,
					);
					const truncatedRequest = new Request(
						`http://actor${newPath}`,
						request,
					);

					const canHibernate =
						canHibernateWebSocket(truncatedRequest);
					return canHibernate;
				} catch (error) {
					logger().error({
						msg: "error calling canHibernateWebSocket",
						error,
					});
					return false;
				}
			} else {
				return false;
			}
		} else if (path === PATH_INSPECTOR_CONNECT) {
			return false;
		} else {
			logger().warn({
				msg: "unexpected path for getActorHibernationConfig",
				path,
			});
			return false;
		}
	}

	async #hwsLoadAll(
		actorId: string,
	): Promise<HibernatingWebSocketMetadata[]> {
		const actor = await this.loadActor(actorId);
		if (!isStaticActorInstance(actor)) {
			const runtime = this.#dynamicRuntimes.get(actorId);
			if (!runtime) {
				return [];
			}
			const entries = await runtime.getHibernatingWebSockets();
			return entries.map((entry) => ({
				gatewayId: entry.gatewayId,
				requestId: entry.requestId,
				rivetMessageIndex: entry.serverMessageIndex,
				envoyMessageIndex: entry.clientMessageIndex,
				path: entry.path,
				headers: entry.headers,
			}));
		}
		return actor.getHibernatingWebSocketMetadata().map((entry) => ({
			gatewayId: entry.gatewayId,
			requestId: entry.requestId,
			rivetMessageIndex: entry.serverMessageIndex,
			envoyMessageIndex: entry.clientMessageIndex,
			path: entry.path,
			headers: entry.headers,
		}));
	}

	async onBeforeActorStart(actor: AnyStaticActorInstance): Promise<void> {
		// Resolve promise if waiting.
		//
		// The websocket restore path needs to be able to load the actor while
		// rebinding persisted sockets, so this promise cannot wait on restore.
		const handler = this.#actors.get(actor.id);
		invariant(handler, "missing actor handler in onBeforeActorReady");
		handler.actorStartError = undefined;
		handler.actorStartPromise?.resolve();
		handler.actorStartPromise = undefined;

		// Restore hibernating requests
		const metaEntries = await this.#hwsLoadAll(actor.id);
		await this.#envoy.restoreHibernatingRequests(actor.id, metaEntries);
	}

}
