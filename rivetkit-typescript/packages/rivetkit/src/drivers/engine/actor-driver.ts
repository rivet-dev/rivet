import type {
	ActorConfig as EngineActorConfig,
	RunnerConfig as EngineRunnerConfig,
	HibernatingWebSocketMetadata,
} from "@rivetkit/engine-runner";
import type { SqliteVfs } from "@rivetkit/sqlite-vfs";
import { idToStr, Runner } from "@rivetkit/engine-runner";
import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import { streamSSE } from "hono/streaming";
import { WSContext, type WSContextInit } from "hono/ws";
import invariant from "invariant";
import { CONN_STATE_MANAGER_SYMBOL } from "@/actor/conn/mod";
import { isStaticActorDefinition, lookupInRegistry } from "@/actor/definition";
import {
	isStaticActorInstance,
	type AnyStaticActorInstance,
} from "@/actor/instance/mod";
import { KEYS } from "@/actor/instance/keys";
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
import {
	type ActorDriver,
	type AnyActorInstance,
	getInitialActorKvState,
	type ManagerDriver,
} from "@/driver-helpers/mod";
import { DynamicActorInstance } from "@/dynamic/instance";
import {
	buildDynamicRuntimeConfigBridge,
	DynamicActorIsolateRuntime,
} from "@/dynamic/isolate-runtime";
import { isDynamicActorDefinition } from "@/dynamic/internal";
import { buildActorNames, type RegistryConfig } from "@/registry/config";
import { getEndpoint } from "@/remote-manager-driver/api-utils";
import {
	type LongTimeoutHandle,
	promiseWithResolvers,
	setLongTimeout,
	stringifyError,
	VERSION,
} from "@/utils";
import { logger } from "./log";

const RUNNER_SSE_PING_INTERVAL = 1000;
const RUNNER_STOP_WAIT_MS = 15_000;
const REMOTE_ACK_HOOK_QUERY_PARAM = "__rivetkitAckHook";

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
	#managerDriver: ManagerDriver;
	#inlineClient: Client<any>;
	#runner: Runner;
	#actors: Map<string, ActorHandler> = new Map();
	#dynamicRuntimes = new Map<string, DynamicActorIsolateRuntime>();
	#hibernatableWebSocketAcks = new Map<
		string,
		HibernatableWebSocketAckState
	>();
	#actorRouter: ActorRouter;

	#runnerStarted: PromiseWithResolvers<undefined> = promiseWithResolvers(
		(reason) =>
			logger().warn({
				msg: "unhandled runner started promise rejection",
				reason,
			}),
	);
	#runnerStopped: PromiseWithResolvers<undefined> = promiseWithResolvers(
		(reason) =>
			logger().warn({
				msg: "unhandled runner stopped promise rejection",
				reason,
			}),
	);
	#isRunnerStopped: boolean = false;

	// HACK: Track actor stop intent locally since the runner protocol doesn't
	// pass the stop reason to onActorStop. This will be fixed when the runner
	// protocol is updated to send the intent directly (see RVT-5284)
	#actorStopIntent: Map<string, "sleep" | "destroy"> = new Map();

	constructor(
		config: RegistryConfig,
		managerDriver: ManagerDriver,
		inlineClient: Client<any>,
	) {
		this.#config = config;
		this.#managerDriver = managerDriver;
		this.#inlineClient = inlineClient;

		// HACK: Override inspector token (which are likely to be
		// removed later on) with token from x-rivet-token header
		const token = config.token;
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

		// Create runner configuration
		const engineRunnerConfig: EngineRunnerConfig = {
			version: config.runner.version,
			endpoint: getEndpoint(config),
			token,
			namespace: config.namespace,
			totalSlots: config.runner.totalSlots,
			runnerName: config.runner.runnerName,
			runnerKey: config.runner.runnerKey ?? crypto.randomUUID(),
			metadata: {
				rivetkit: { version: VERSION },
			},
			prepopulateActorNames: buildActorNames(config),
			onConnected: () => {
				this.#runnerStarted.resolve(undefined);
			},
			onDisconnected: (_code, _reason) => {},
			onShutdown: () => {
				this.#runnerStopped.resolve(undefined);
				this.#isRunnerStopped = true;
			},
			fetch: this.#runnerFetch.bind(this),
			websocket: this.#runnerWebSocket.bind(this),
			hibernatableWebSocket: {
				canHibernate: this.#hwsCanHibernate.bind(this),
			},
			onActorStart: this.#runnerOnActorStart.bind(this),
			onActorStop: this.#runnerOnActorStop.bind(this),
			logger: getLogger("engine-runner"),
		};

		// Create and start runner
		this.#runner = new Runner(engineRunnerConfig);
		this.#runner.start();
		logger().debug({
			msg: "engine runner started",
			endpoint: config.endpoint,
			namespace: config.namespace,
			runnerName: config.runner.runnerName,
		});
	}

	getExtraActorLogParams(): Record<string, string> {
		return { runnerId: this.#runner.runnerId ?? "-" };
	}

	#hibernatableWebSocketAckKey(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
	): string {
		return `${idToStr(gatewayId)}:${idToStr(requestId)}`;
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

	#isDynamicActor(actorId: string): boolean {
		return this.#dynamicRuntimes.has(actorId);
	}

	#getDynamicRuntime(actorId: string): DynamicActorIsolateRuntime {
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
		this.#runner.setAlarm(actor.id, timestamp);
	}

	// No database overrides - will use KV-backed implementation from rivetkit/db

	// MARK: - Batch KV operations
	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		await this.#runner.kvPut(actorId, entries);
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		return await this.#runner.kvGet(actorId, keys);
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		await this.#runner.kvDelete(actorId, keys);
	}

	async kvList(actorId: string): Promise<Uint8Array[]> {
		const entries = await this.#runner.kvListPrefix(
			actorId,
			new Uint8Array(),
		);
		const keys = entries.map(([key]) => key);
		return keys;
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#runner.kvListPrefix(actorId, prefix);
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
		this.#runner.sendHibernatableWebSocketMessageAck(
			gatewayId,
			requestId,
			serverMessageIndex,
		);
	}

	/** Creates a SQLite VFS instance for creating KV-backed databases */
	async createSqliteVfs(): Promise<SqliteVfs> {
		// Dynamic import keeps @rivetkit/sqlite out of the main entrypoint bundle.
		// Returning a fresh SqliteVfs gives each actor an isolated sqlite module
		// instance, avoiding async re-entrancy across actors.
		const specifier = "@rivetkit/" + "sqlite-vfs";
		const { SqliteVfs } = await import(specifier);
		return new SqliteVfs();
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
		this.#runner.sleepActor(actorId);
	}

	startDestroy(actorId: string) {
		// HACK: Track intent for onActorStop (see RVT-5284)
		this.#actorStopIntent.set(actorId, "destroy");
		this.#runner.stopActor(actorId);
	}

	async shutdownRunner(immediate: boolean): Promise<void> {
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

			const actorSleepDeadline = Date.now() + RUNNER_STOP_WAIT_MS;
			while (this.#actors.size > 0 && Date.now() < actorSleepDeadline) {
				await new Promise((resolve) => setTimeout(resolve, 50));
			}

			if (this.#actors.size > 0) {
				logger().warn({
					msg: "timed out waiting for actors to stop before runner drain",
					remainingActors: this.#actors.size,
					waitMs: RUNNER_STOP_WAIT_MS,
				});
			} else {
				logger().debug({
					msg: "all actors stopped before runner drain",
				});
			}
		}

		try {
			await this.#runner.shutdown(immediate);
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
			this.#runnerStopped.promise.then(() => true),
			new Promise<false>((resolve) =>
				setTimeout(() => resolve(false), RUNNER_STOP_WAIT_MS),
			),
		]);
		if (!stopped) {
			logger().warn({
				msg: "timed out waiting for runner shutdown",
				waitMs: RUNNER_STOP_WAIT_MS,
			});
		}

		this.#dynamicRuntimes.clear();
	}

	async serverlessHandleStart(c: HonoContext): Promise<Response> {
		return streamSSE(c, async (stream) => {
			// NOTE: onAbort does not work reliably
			stream.onAbort(() => {});
			c.req.raw.signal.addEventListener("abort", () => {
				logger().debug("SSE aborted, shutting down runner");

				// We cannot assume that the request will always be closed gracefully by Rivet. We always proceed with a graceful shutdown in case the request was terminated for any other reason.
				//
				// If we did not use a graceful shutdown, the runner would
				this.shutdownRunner(false);
			});

			await this.#runnerStarted.promise;

			// Runner id should be set if the runner started
			const payload = this.#runner.getServerlessInitPacket();
			invariant(payload, "runnerId not set");
			await stream.writeSSE({ data: payload });

			// Send ping every second to keep the connection alive
			while (true) {
				if (this.#isRunnerStopped) {
					logger().debug({
						msg: "runner is stopped",
					});
					break;
				}

				if (stream.closed || stream.aborted) {
					logger().debug({
						msg: "runner sse stream closed",
						closed: stream.closed,
						aborted: stream.aborted,
					});
					break;
				}

				await stream.writeSSE({ event: "ping", data: "" });
				await stream.sleep(RUNNER_SSE_PING_INTERVAL);
			}

			// Wait for the runner to stop if the SSE stream aborted early for any reason
			await this.#runnerStopped.promise;
		});
	}

	async #runnerOnActorStart(
		actorId: string,
		generation: number,
		actorConfig: EngineActorConfig,
	): Promise<void> {
		logger().debug({
			msg: "runner actor starting",
			actorId,
			name: actorConfig.name,
			key: actorConfig.key,
			generation,
		});

		// Deserialize input
		let input: any;
		if (actorConfig.input) {
			input = cbor.decode(actorConfig.input);
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
			// Initialize storage
			const [persistDataBuffer] = await this.#runner.kvGet(actorId, [
				KEYS.PERSIST_DATA,
			]);
			if (persistDataBuffer === null) {
				const initialKvState = getInitialActorKvState(input);
				await this.#runner.kvPut(actorId, initialKvState);
			}

			// Create actor instance
			const definition = lookupInRegistry(this.#config, actorConfig.name);
			if (isDynamicActorDefinition(definition)) {
				if (this.#dynamicRuntimes.has(actorId)) {
					throw new Error(
						`dynamic runtime unexpectedly already loaded before actor start for ${actorId}`,
					);
				}
				const runtime = new DynamicActorIsolateRuntime({
					actorId,
					actorName: name,
					actorKey: key,
					runtimeConfig: buildDynamicRuntimeConfigBridge(
						this.#config,
					),
					input,
					region: "unknown",
					loader: definition.loader,
					auth: definition.auth,
					actorDriver: this,
					inlineClient: this.#inlineClient,
				});
				await runtime.start();
				this.#dynamicRuntimes.set(actorId, runtime);
				await runtime.ensureStarted();

				const dynamicActor = new DynamicActorInstance(actorId, runtime);
				handler.actor = dynamicActor;

				handler.actorStartError = undefined;
				handler.actorStartPromise?.resolve();
				handler.actorStartPromise = undefined;

				const metaEntries =
					await dynamicActor.getHibernatingWebSockets();
				await this.#runner.restoreHibernatingRequests(
					actorId,
					metaEntries,
				);
			} else if (isStaticActorDefinition(definition)) {
				const staticActor =
					(await definition.instantiate()) as AnyStaticActorInstance;
				handler.actor = staticActor;

				// Apply protocol limits as per-instance overrides without mutating the shared definition
				const protocolMetadata = this.#runner.getProtocolMetadata();
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
				);
			} else {
				throw new Error(
					`actor definition for ${actorConfig.name} is not instantiable`,
				);
			}

			logger().debug({ msg: "runner actor started", actorId, name, key });
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
				msg: "runner actor failed to start",
				actorId,
				name,
				key,
				err: stringifyError(error),
			});

			try {
				this.#runner.stopActor(actorId);
			} catch (stopError) {
				logger().debug({
					msg: "failed to stop actor after start failure",
					actorId,
					err: stringifyError(stopError),
				});
			}
		}
	}

	async #runnerOnActorStop(
		actorId: string,
		generation: number,
	): Promise<void> {
		logger().debug({ msg: "runner actor stopping", actorId, generation });

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
				msg: "no runner actor handler to stop",
				actorId,
				reason,
			});
			return;
		}

		if (handler.actorStartPromise) {
			try {
				logger().debug({
					msg: "runner actor stopping before it started, waiting",
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
				await handler.actor.onStop(reason);
			} catch (err) {
				logger().error({
					msg: "error in onStop, proceeding with removing actor",
					err: stringifyError(err),
				});
			}
		}
		this.#dynamicRuntimes.delete(actorId);

		this.#actors.delete(actorId);

		logger().debug({ msg: "runner actor stopped", actorId, reason });
	}

	// MARK: - Runner Networking
	async #runnerFetch(
		_runner: Runner,
		actorId: string,
		_gatewayIdBuf: ArrayBuffer,
		_requestIdBuf: ArrayBuffer,
		request: Request,
	): Promise<Response> {
		logger().debug({
			msg: "runner fetch",
			actorId,
			url: request.url,
			method: request.method,
		});
		if (this.#isDynamicActor(actorId)) {
			return await this.#getDynamicRuntime(actorId).fetch(request);
		}
		return await this.#actorRouter.fetch(request, { actorId });
	}

	async #runnerWebSocket(
		_runner: Runner,
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

		websocket.addEventListener("open", (event) => {
			wsHandler.onOpen(event, wsContext);
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

			// Check if actor is stopping - if so, don't process new messages.
			// These messages will be reprocessed when the actor wakes up from hibernation.
			// TODO: This will never retransmit the socket and the socket will close
			if (actor?.isStopping) {
				logger().debug({
					msg: "ignoring ws message, actor is stopping",
					connId: conn?.id,
					actorId: actor?.id,
					messageIndex: event.rivetMessageIndex,
				});
				return;
			}

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
		});

		websocket.addEventListener("close", (event) => {
			wsHandler.onClose(event, wsContext);
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

			// NOTE: Persisted connection is removed when `conn.disconnect`
			// is called by the WebSocket route
		});

		websocket.addEventListener("error", (event) => {
			wsHandler.onError(event, wsContext);
		});
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
			runtime = this.#getDynamicRuntime(actorId);
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

		// Get actor instance from runner to access actor name
		const actorInstance = this.#runner.getActor(actorId);
		if (!actorInstance) {
			logger().warn({
				msg: "actor not found in #hwsCanHibernate",
				actorId,
			});
			return false;
		}

		const handler = this.#actors.get(actorId);

		// Determine configuration for new WS
		logger().debug({
			msg: "no existing hibernatable websocket found",
			gatewayId: idToStr(gatewayId),
			requestId: idToStr(requestId),
		});
		if (path === PATH_CONNECT) {
			return true;
		} else if (
			path === PATH_WEBSOCKET_BASE ||
			path.startsWith(PATH_WEBSOCKET_PREFIX)
		) {
			// Find actor config
			// Hibernation capability is a definition-level property, so the
			// runner can decide it before the actor has fully started.
			const actorName =
				"config" in actorInstance &&
				actorInstance.config &&
				typeof actorInstance.config === "object" &&
				"name" in actorInstance.config &&
				typeof actorInstance.config.name === "string"
					? actorInstance.config.name
					: this.#actors.get(actorId)?.actorName;
			invariant(
				actorName,
				`missing actor name for hibernatable websocket actor ${actorId}`,
			);
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
				serverMessageIndex: entry.serverMessageIndex,
				clientMessageIndex: entry.clientMessageIndex,
				path: entry.path,
				headers: entry.headers,
			}));
		}
		return actor.getHibernatingWebSocketMetadata().map((entry) => ({
			gatewayId: entry.gatewayId,
			requestId: entry.requestId,
			serverMessageIndex: entry.serverMessageIndex,
			clientMessageIndex: entry.clientMessageIndex,
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
		await this.#runner.restoreHibernatingRequests(actor.id, metaEntries);
	}
}
