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
import { type AnyConn, CONN_STATE_MANAGER_SYMBOL } from "@/actor/conn/mod";
import { lookupInRegistry } from "@/actor/definition";
import { KEYS } from "@/actor/instance/keys";
import { deserializeActorKey } from "@/actor/keys";
import { getValueLength } from "@/actor/protocol/old";
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
	actorStartPromise?: ReturnType<typeof promiseWithResolvers<void>>;
	actorStartError?: Error;
	alarmTimeout?: LongTimeoutHandle;
}

export type DriverContext = {};

export class EngineActorDriver implements ActorDriver {
	#config: RegistryConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: Client<any>;
	#runner: Runner;
	#actors: Map<string, ActorHandler> = new Map();
	#actorRouter: ActorRouter;

	#runnerStarted: PromiseWithResolvers<undefined> = promiseWithResolvers((reason) => logger().warn({ msg: "unhandled runner started promise rejection", reason }));
	#runnerStopped: PromiseWithResolvers<undefined> = promiseWithResolvers((reason) => logger().warn({ msg: "unhandled runner stopped promise rejection", reason }));
	#isRunnerStopped: boolean = false;

	// HACK: Track actor stop intent locally since the runner protocol doesn't
	// pass the stop reason to onActorStop. This will be fixed when the runner
	// protocol is updated to send the intent directly (see RVT-5284)
	#actorStopIntent: Map<string, "sleep" | "destroy"> = new Map();

	// Map of conn IDs to message index waiting to be persisted before sending
	// an ack
	//
	// serverMessageIndex is updated and pendingAck is flagged in needed in
	// onBeforePersistConnect, then the HWS ack message is sent in
	// onAfterPersistConn. This allows us to track what's about to be written
	// to storage to prevent race conditions with the serverMessageIndex being
	// updated while writing the existing state.
	//
	// bufferedMessageSize tracks the total bytes received since last persist
	// to force a saveState when threshold is reached. This is the amount of
	// data currently buffered on the gateway.
	#hwsMessageIndex = new Map<
		string,
		{
			serverMessageIndex: number;
			bufferedMessageSize: number;
			pendingAckFromMessageIndex: boolean;
			pendingAckFromBufferSize: boolean;
		}
	>();

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

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		const handler = this.#actors.get(actor.id);
		if (!handler) {
			logger().warn({
				msg: "no handler for actor to set alarm",
			});

			return;
		}

		// Clear prev timeout
		if (handler.alarmTimeout) {
			handler.alarmTimeout.abort();
			handler.alarmTimeout = undefined;
		}

		// Set alarm
		const delay = Math.max(0, timestamp - Date.now());
		handler.alarmTimeout = setLongTimeout(() => {
			actor.onAlarm();
			handler.alarmTimeout = undefined;
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
		logger().info({
			msg: "kvList called",
			actorId,
			keysCount: keys.length,
			keys: keys.map((k) => new TextDecoder().decode(k)),
		});
		return keys;
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		const result = await this.#runner.kvListPrefix(actorId, prefix);
		logger().info({
			msg: "kvListPrefix called",
			actorId,
			prefixStr: new TextDecoder().decode(prefix),
			entriesCount: result.length,
			keys: result.map(([key]) => new TextDecoder().decode(key)),
		});
		return result;
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

		// TODO: We need to update the runner to have a draining state so:
		// 1. Send ToServerDraining
		//		- This causes Pegboard to stop allocating actors to this runner
		// 2. Pegboard sends ToClientStopActor for all actors on this runner which handles the graceful migration of each actor independently
		// 3. Send ToServerStopping once all actors have successfully stopped
		//
		// What's happening right now is:
		// 1. All actors enter stopped state
		// 2. Actors still respond to requests because only RivetKit knows it's
		//    stopping, this causes all requests to issue errors that the actor is
		//    stopping. (This will NOT return a 503 bc the runner has no idea the
		//    actors are stopping.)
		// 3. Once the last actor stops, then the runner finally stops + actors
		//    reschedule
		//
		// This means that:
		// - All actors on this runner are bricked until the slowest onStop finishes
		// - Guard will not gracefully handle requests bc it's not receiving a 503
		// - Actors can still be scheduled to this runner while the other
		//   actors are stopping, meaning that those actors will NOT get onStop
		//   and will potentiall corrupt their state
		//
		// HACK: Stop all actors to allow state to be saved
		// NOTE: onStop is only supposed to be called by the runner, we're
		// abusing it here
		logger().debug({
			msg: "stopping all actors before shutdown",
			actorCount: this.#actors.size,
		});
		const stopPromises: Promise<void>[] = [];
		for (const [_actorId, handler] of this.#actors.entries()) {
			if (handler.actor) {
				stopPromises.push(
					handler.actor.onStop("sleep").catch((err) => {
						handler.actor?.rLog.error({
							msg: "onStop errored",
							error: stringifyError(err),
						});
					}),
				);
			}
		}
		await Promise.all(stopPromises);
		logger().debug({ msg: "all actors stopped" });

		try {
			await this.#runner.shutdown(immediate);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			if (message.includes("WebSocket connection closed during shutdown")) {
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
				actorStartPromise: promiseWithResolvers((reason) => logger().warn({ msg: "unhandled actor start promise rejection", reason })),
			};
			this.#actors.set(actorId, handler);
		}
		handler.actorStartError = undefined;

		const name = actorConfig.name as string;
		invariant(actorConfig.key, "actor should have a key");
		const key = deserializeActorKey(actorConfig.key);

		try {
			// Initialize storage
			const [persistDataBuffer] = await this.#runner.kvGet(actorId, [
				KEYS.PERSIST_DATA,
			]);
			if (persistDataBuffer === null) {
				const initialKvState = getInitialActorKvState(input);
				await this.#runner.kvPut(actorId, initialKvState);
				logger().debug({
					msg: "initialized persist data for new actor",
					actorId,
				});
			} else {
				logger().debug({
					msg: "found existing persist data for actor",
					actorId,
					dataSize: persistDataBuffer.byteLength,
				});
			}

			// Create actor instance
			const definition = lookupInRegistry(this.#config, actorConfig.name);
			handler.actor = await definition.instantiate();

			// Start actor
			await handler.actor.start(
				this,
				this.#inlineClient,
				actorId,
				name,
				key,
				"unknown", // TODO: Add regions
			);

			logger().debug({ msg: "runner actor started", actorId, name, key });
		} catch (innerError) {
			const error =
				innerError instanceof Error
					? new Error(
							`Failed to start actor ${actorId}: ${innerError.message}`,
							{ cause: innerError },
						)
					: new Error(`Failed to start actor ${actorId}: ${String(innerError)}`);
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
		if (handler?.actorStartPromise) {
			const startError =
				handler.actorStartError ??
				new Error(`Actor ${actorId} stopped before start completed`);
			handler.actorStartError = startError;
			handler.actorStartPromise.reject(startError);
			handler.actorStartPromise = undefined;
		}
		if (handler?.actor) {
			try {
				await handler.actor.onStop(reason);
			} catch (err) {
				logger().error({
					msg: "error in onStop, proceeding with removing actor",
					err: stringifyError(err),
				});
			}
		}
		if (handler) this.#actors.delete(actorId);

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

		// Add a unique ID to track this WebSocket object
		const wsUniqueId = `ws_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
		(websocket as any).__rivet_ws_id = wsUniqueId;

		logger().debug({
			msg: "runner websocket",
			actorId,
			url: request.url,
			isRestoringHibernatable,
			websocketObjectId: websocketRaw
				? Object.prototype.toString.call(websocketRaw)
				: "null",
			websocketType: websocketRaw?.constructor?.name,
			wsUniqueId,
			websocketProps: websocketRaw
				? Object.keys(websocketRaw).join(", ")
				: "null",
		});

		// Parse configuration from Sec-WebSocket-Protocol header (optional for path-based routing)
		const protocols = request.headers.get("sec-websocket-protocol");
		const { encoding, connParams } = parseWebSocketProtocols(protocols);

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

		// Log when attaching event listeners
		logger().debug({
			msg: "attaching websocket event listeners",
			actorId,
			connId: conn?.id,
			wsUniqueId: (websocket as any).__rivet_ws_id,
			isRestoringHibernatable,
			websocketType: websocket?.constructor?.name,
		});

		if (isRestoringHibernatable) {
			wsHandler.onRestore?.(wsContext);
		}

		websocket.addEventListener("open", (event) => {
			wsHandler.onOpen(event, wsContext);
		});

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			logger().debug({
				msg: "websocket message event listener triggered",
				connId: conn?.id,
				actorId: actor?.id,
				messageIndex: event.rivetMessageIndex,
				hasWsHandler: !!wsHandler,
				hasOnMessage: !!wsHandler?.onMessage,
				actorIsStopping: actor?.isStopping,
				websocketType: websocket?.constructor?.name,
				wsUniqueId: (websocket as any).__rivet_ws_id,
				eventTargetWsId: (event.target as any)?.__rivet_ws_id,
			});

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
			logger().debug({
				msg: "calling wsHandler.onMessage",
				connId: conn?.id,
				messageIndex: event.rivetMessageIndex,
			});
			wsHandler.onMessage(event, wsContext);

			// Persist message index for hibernatable connections
			const hibernate = connStateManager?.hibernatableData;

			if (hibernate && conn && actor) {
				invariant(
					typeof event.rivetMessageIndex === "number",
					"missing event.rivetMessageIndex",
				);

				// Persist message index
				const previousMsgIndex = hibernate.serverMessageIndex;
				hibernate.serverMessageIndex = event.rivetMessageIndex;
				logger().info({
					msg: "persisting message index",
					connId: conn.id,
					previousMsgIndex,
					newMsgIndex: event.rivetMessageIndex,
				});

				// Calculate message size and track cumulative size
				const entry = this.#hwsMessageIndex.get(conn.id);
				if (entry) {
					// Track message length
					const messageLength = getValueLength(event.data);
					entry.bufferedMessageSize += messageLength;

					if (
						entry.bufferedMessageSize >=
						CONN_BUFFERED_MESSAGE_SIZE_THRESHOLD
					) {
						// Reset buffered message size immeidatley (instead
						// of waiting for onAfterPersistConn) since we may
						// receive more messages before onAfterPersistConn
						// is called, which would called saveState
						// immediate multiple times
						entry.bufferedMessageSize = 0;
						entry.pendingAckFromBufferSize = true;

						// Save state immediately if approaching buffer threshold
						actor.stateManager.saveState({
							immediate: true,
						});
					} else {
						// Save message index. The maxWait is set to the ack deadline
						// since we ack the message immediately after persisting the index.
						// If cumulative size exceeds threshold, force immediate persist.
						//
						// This will call EngineActorDriver.onAfterPersistConn after
						// persist to send the ack to the gateway.
						actor.stateManager.saveState({
							maxWait: CONN_MESSAGE_ACK_DEADLINE,
						});
					}
				} else {
					// Fallback if entry missing
					actor.stateManager.saveState({
						maxWait: CONN_MESSAGE_ACK_DEADLINE,
					});
				}
			}
		});

		websocket.addEventListener("close", (event) => {
			wsHandler.onClose(event, wsContext);

			// NOTE: Persisted connection is removed when `conn.disconnect`
			// is called by the WebSocket route
		});

		websocket.addEventListener("error", (event) => {
			wsHandler.onError(event, wsContext);
		});

		// Log event listener attachment for restored connections
		if (isRestoringHibernatable) {
			logger().info({
				msg: "event listeners attached to restored websocket",
				actorId,
				connId: conn?.id,
				gatewayId: idToStr(gatewayIdBuf),
				requestId: idToStr(requestIdBuf),
				websocketType: websocket?.constructor?.name,
				hasMessageListener: !!websocket.addEventListener,
			});
		}
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

		// Load actor handler to access persisted data
		const handler = this.#actors.get(actorId);
		if (!handler) {
			logger().warn({
				msg: "actor handler not found in #hwsCanHibernate",
				actorId,
			});
			return false;
		}
		if (!handler.actor) {
			logger().warn({
				msg: "actor not found in #hwsCanHibernate",
				actorId,
			});
			return false;
		}

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
			const definition = lookupInRegistry(
				this.#config,
				actorInstance.config.name,
			);

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
		return actor.conns
			.values()
			.map((conn) => {
				const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
				const hibernatable = connStateManager.hibernatableData;
				if (!hibernatable) return undefined;
				return {
					gatewayId: hibernatable.gatewayId,
					requestId: hibernatable.requestId,
					serverMessageIndex: hibernatable.serverMessageIndex,
					clientMessageIndex: hibernatable.clientMessageIndex,
					path: hibernatable.requestPath,
					headers: hibernatable.requestHeaders,
				} satisfies HibernatingWebSocketMetadata;
			})
			.filter((x) => x !== undefined)
			.toArray();
	}

	async onBeforeActorStart(actor: AnyActorInstance): Promise<void> {
		// Resolve promise if waiting
		const handler = this.#actors.get(actor.id);
		invariant(handler, "missing actor handler in onBeforeActorReady");
		handler.actorStartError = undefined;
		handler.actorStartPromise?.resolve();
		handler.actorStartPromise = undefined;

		// Restore hibernating requests
		const metaEntries = await this.#hwsLoadAll(actor.id);
		await this.#runner.restoreHibernatingRequests(actor.id, metaEntries);
	}

	onCreateConn(conn: AnyConn) {
		const hibernatable = conn[CONN_STATE_MANAGER_SYMBOL].hibernatableData;
		if (!hibernatable) return;

		this.#hwsMessageIndex.set(conn.id, {
			serverMessageIndex: hibernatable.serverMessageIndex,
			bufferedMessageSize: 0,
			pendingAckFromMessageIndex: false,
			pendingAckFromBufferSize: false,
		});

		logger().debug({
			msg: "created #hwsMessageIndex entry",
			connId: conn.id,
			serverMessageIndex: hibernatable.serverMessageIndex,
		});
	}

	onDestroyConn(conn: AnyConn) {
		this.#hwsMessageIndex.delete(conn.id);

		logger().debug({
			msg: "removed #hwsMessageIndex entry",
			connId: conn.id,
		});
	}

	onBeforePersistConn(conn: AnyConn) {
		const stateManager = conn[CONN_STATE_MANAGER_SYMBOL];
		const hibernatable = stateManager.hibernatableDataOrError();

		const entry = this.#hwsMessageIndex.get(conn.id);
		if (!entry) {
			logger().warn({
				msg: "missing EngineActorDriver.#hwsMessageIndex entry for conn",
				connId: conn.id,
			});
			return;
		}

		// There is a newer message index
		entry.pendingAckFromMessageIndex =
			hibernatable.serverMessageIndex > entry.serverMessageIndex;
		entry.serverMessageIndex = hibernatable.serverMessageIndex;
	}

	onAfterPersistConn(conn: AnyConn) {
		const stateManager = conn[CONN_STATE_MANAGER_SYMBOL];
		const hibernatable = stateManager.hibernatableDataOrError();

		const entry = this.#hwsMessageIndex.get(conn.id);
		if (!entry) {
			logger().warn({
				msg: "missing EngineActorDriver.#hwsMessageIndex entry for conn",
				connId: conn.id,
			});
			return;
		}

		// Ack entry
		if (
			entry.pendingAckFromMessageIndex ||
			entry.pendingAckFromBufferSize
		) {
			this.#runner.sendHibernatableWebSocketMessageAck(
				hibernatable.gatewayId,
				hibernatable.requestId,
				entry.serverMessageIndex,
			);
			entry.pendingAckFromMessageIndex = false;
			entry.pendingAckFromBufferSize = false;
			entry.bufferedMessageSize = 0;
		}
	}
}
