import type {
	ActorConfig as EngineActorConfig,
	RunnerConfig as EngineRunnerConfig,
	HibernatingWebSocketMetadata,
} from "@rivetkit/engine-runner";
import { Runner } from "@rivetkit/engine-runner";
import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import { streamSSE } from "hono/streaming";
import { WSContext } from "hono/ws";
import invariant from "invariant";
import { lookupInRegistry } from "@/actor/definition";
import { KEYS } from "@/actor/instance/kv";
import { deserializeActorKey } from "@/actor/keys";
import { type ActorRouter, createActorRouter } from "@/actor/router";
import {
	handleRawWebSocket,
	handleWebSocketConnect,
	parseWebSocketProtocols,
	truncateRawWebSocketPathPrefix,
} from "@/actor/router-endpoints";
import type { Client } from "@/client/client";
import {
	PATH_CONNECT,
	PATH_WEBSOCKET_PREFIX,
} from "@/common/actor-router-consts";
import type { UpgradeWebSocketArgs } from "@/common/inline-websocket-adapter2";
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
import { CONN_STATE_MANAGER_SYMBOL, type AnyConn } from "@/actor/conn/mod";
import { buildActorNames, type RegistryConfig } from "@/registry/config";
import type { RunnerConfig } from "@/registry/run-config";
import { getEndpoint } from "@/remote-manager-driver/api-utils";
import {
	arrayBuffersEqual,
	idToStr,
	type LongTimeoutHandle,
	promiseWithResolvers,
	setLongTimeout,
	stringifyError,
} from "@/utils";
import { logger } from "./log";
import { RequestId } from "@/schemas/actor-persist/mod";

const RUNNER_SSE_PING_INTERVAL = 1000;

interface ActorHandler {
	actor?: AnyActorInstance;
	actorStartPromise?: ReturnType<typeof promiseWithResolvers<void>>;
}

export type DriverContext = {};

export class EngineActorDriver implements ActorDriver {
	#registryConfig: RegistryConfig;
	#runConfig: RunnerConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: Client<any>;
	#runner: Runner;
	#actors: Map<string, ActorHandler> = new Map();
	#actorRouter: ActorRouter;
	#version: number = 1; // Version for the runner protocol
	#alarmTimeout?: LongTimeoutHandle;

	#runnerStarted: PromiseWithResolvers<undefined> = promiseWithResolvers();
	#runnerStopped: PromiseWithResolvers<undefined> = promiseWithResolvers();
	#isRunnerStopped: boolean = false;

	// HACK: Track actor stop intent locally since the runner protocol doesn't
	// pass the stop reason to onActorStop. This will be fixed when the runner
	// protocol is updated to send the intent directly (see RVT-5284)
	#actorStopIntent: Map<string, "sleep" | "destroy"> = new Map();

	// Request IDs that are waiting to be acknowledged after the next persist
	//
	// We store the RequestId since it's the array buffer version we need to
	// pass back to the runner.
	#hibernatableWebSocketAckQueue = new Map<
		string,
		{ actorId: string; requestId: RequestId; messageIndex: number }
	>();

	constructor(
		registryConfig: RegistryConfig,
		runConfig: RunnerConfig,
		managerDriver: ManagerDriver,
		inlineClient: Client<any>,
	) {
		this.#registryConfig = registryConfig;
		this.#runConfig = runConfig;
		this.#managerDriver = managerDriver;
		this.#inlineClient = inlineClient;

		// HACK: Override inspector token (which are likely to be
		// removed later on) with token from x-rivet-token header
		const token = runConfig.token;
		if (token && runConfig.inspector && runConfig.inspector.enabled) {
			runConfig.inspector.token = () => token;
		}

		this.#actorRouter = createActorRouter(
			runConfig,
			this,
			registryConfig.test.enabled,
		);

		// Create runner configuration
		const engineRunnerConfig: EngineRunnerConfig = {
			version: this.#version,
			endpoint: getEndpoint(runConfig),
			token,
			namespace: runConfig.namespace,
			totalSlots: runConfig.totalSlots,
			runnerName: runConfig.runnerName,
			runnerKey: runConfig.runnerKey ?? crypto.randomUUID(),
			metadata: {
				inspectorToken: this.#runConfig.inspector.token(),
			},
			prepopulateActorNames: buildActorNames(registryConfig),
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
				loadAll: this.#hwsLoadAll.bind(this),
				persistMessageIndex: this.#hwsPersistMessageIndex.bind(this),
				removePersisted: this.#hwsRemovePersisted.bind(this),
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
			endpoint: runConfig.endpoint,
			namespace: runConfig.namespace,
			runnerName: runConfig.runnerName,
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
		if (!handler.actor) throw new Error("Actor should be loaded");
		return handler;
	}

	getContext(actorId: string): DriverContext {
		return {};
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		// Clear prev timeout
		if (this.#alarmTimeout) {
			this.#alarmTimeout.abort();
			this.#alarmTimeout = undefined;
		}

		// Set alarm
		const delay = Math.max(0, timestamp - Date.now());
		this.#alarmTimeout = setLongTimeout(() => {
			actor.onAlarm();
			this.#alarmTimeout = undefined;
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

	async getDatabase(_actorId: string): Promise<unknown | undefined> {
		return undefined;
	}

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

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#runner.kvListPrefix(actorId, prefix);
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

		await this.#runner.shutdown(immediate);
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
				actorStartPromise: promiseWithResolvers(),
			};
			this.#actors.set(actorId, handler);
		}

		const name = actorConfig.name as string;
		invariant(actorConfig.key, "actor should have a key");
		const key = deserializeActorKey(actorConfig.key);

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
		const definition = lookupInRegistry(
			this.#registryConfig,
			actorConfig.name,
		);
		handler.actor = definition.instantiate();

		// Start actor
		await handler.actor.start(
			this,
			this.#inlineClient,
			actorId,
			name,
			key,
			"unknown", // TODO: Add regions
		);

		// Resolve promise if waiting
		handler.actorStartPromise?.resolve();
		handler.actorStartPromise = undefined;

		logger().debug({ msg: "runner actor started", actorId, name, key });
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
		if (handler?.actor) {
			try {
				await handler.actor.onStop(reason);
			} catch (err) {
				logger().error({
					msg: "error in onStop, proceeding with removing actor",
					err: stringifyError(err),
				});
			}
			this.#actors.delete(actorId);
		}

		logger().debug({ msg: "runner actor stopped", actorId, reason });
	}

	// MARK: - Runner Networking
	async #runnerFetch(
		_runner: Runner,
		actorId: string,
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

	#runnerWebSocket(
		_runner: Runner,
		actorId: string,
		websocketRaw: any,
		requestIdBuf: ArrayBuffer,
		request: Request,
	): void {
		const websocket = websocketRaw as UniversalWebSocket;
		const requestId = idToStr(requestIdBuf);

		logger().debug({ msg: "runner websocket", actorId, url: request.url });

		const url = new URL(request.url);

		// Parse configuration from Sec-WebSocket-Protocol header (optional for path-based routing)
		const protocols = request.headers.get("sec-websocket-protocol");
		const { encoding, connParams } = parseWebSocketProtocols(protocols);

		// Fetch WS handler
		//
		// We store the promise since we need to add WebSocket event listeners immediately that will wait for the promise to resolve
		let wsHandlerPromise: Promise<UpgradeWebSocketArgs>;
		if (url.pathname === PATH_CONNECT) {
			wsHandlerPromise = handleWebSocketConnect(
				request,
				this.#runConfig,
				this,
				actorId,
				encoding,
				connParams,
				requestId,
				requestIdBuf,
			);
		} else if (url.pathname.startsWith(PATH_WEBSOCKET_PREFIX)) {
			wsHandlerPromise = handleRawWebSocket(
				request,
				url.pathname + url.search,
				this,
				actorId,
				requestIdBuf,
				connParams,
			);
		} else {
			throw new Error(`Unreachable path: ${url.pathname}`);
		}

		// Connect the Hono WS hook to the adapter
		const wsContext = new WSContext(websocket);

		wsHandlerPromise.catch((err) => {
			logger().error({ msg: "building websocket handlers errored", err });
			wsContext.close(1011, `${err}`);
		});

		if (websocket.readyState === 1) {
			wsHandlerPromise.then((x) =>
				x.onOpen?.(new Event("open"), wsContext),
			);
		} else {
			websocket.addEventListener("open", (event) => {
				wsHandlerPromise.then((x) => x.onOpen?.(event, wsContext));
			});
		}

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			// Process the message after all hibernation logic and validation in case the message is out of order
			wsHandlerPromise.then((x) => x.onMessage?.(event, wsContext));
		});

		websocket.addEventListener("close", (event) => {
			wsHandlerPromise.then((x) => x.onClose?.(event, wsContext));
		});

		websocket.addEventListener("error", (event) => {
			wsHandlerPromise.then((x) => x.onError?.(event, wsContext));
		});
	}

	// MARK: - Hibernating WebSockets
	#hwsCanHibernate(
		actorId: string,
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
			requestId: idToStr(requestId),
		});
		if (path === PATH_CONNECT) {
			return true;
		} else if (path.startsWith(PATH_WEBSOCKET_PREFIX)) {
			// Find actor config
			const definition = lookupInRegistry(
				this.#registryConfig,
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
		} else {
			logger().warn({
				msg: "unexpected path for getActorHibernationConfig",
				path,
			});
			return false;
		}
	}

	#hwsLoadAll(actorId: string): HibernatingWebSocketMetadata[] {
		// TODO: Load actor in a better way
		const actor = this.#actors.get(actorId);
		invariant(actor?.actor, "actor not loaded");

		return actor.actor.conns
			.values()
			.map((conn) => {
				const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
				const hibernatable = connStateManager.hibernatableData;
				if (!hibernatable) return undefined;
				return {
					requestId: hibernatable.hibernatableRequestId,
					path: hibernatable.requestPath,
					headers: hibernatable.requestHeaders,
					messageIndex: hibernatable.msgIndex,
				} satisfies HibernatingWebSocketMetadata;
			})
			.filter((x) => x !== undefined)
			.toArray();
	}

	#hwsPersistMessageIndex(actorId: string, requestId: RequestId) {
		// TODO: is this the right way of getting the actor

		const actor = this.#actors.get(actorId);
		const conn = actor?.actor?.connectionManager.findHibernatableConn(requestId);

		if (!conn) {
			logger().warn({
				msg: "cannot find conn to persist message index to",
				actorId,
				requestId: idToStr(requestId),
			});
			return;
		}

		this.#hibernatableWebSocketAckQueue.set(conn.id, {});

		// TODO: Find conn with request ID
		// TODO: Add conn to persist queue
		// TODO: Start timer to force save
	}

	#hwsRemovePersisted(actorId: string, requestId: RequestId) {
		// // TODO: persist immediately
		// const actorHandler = this.#actors.get(actorId);
		// if (actorHandler?.actor) {
		// 	const hibernatableArray =
		// 		actorHandler.actor.persist.hibernatableConns;
		// 	const wsIndex = hibernatableArray.findIndex((conn: any) =>
		// 		arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
		// 	);
		//
		// 	if (wsIndex !== -1) {
		// 		const removed = hibernatableArray.splice(wsIndex, 1);
		// 		logger().debug({
		// 			msg: "removed hibernatable websocket",
		// 			requestId,
		// 			actorId,
		// 			removedMsgIndex:
		// 				removed[0]?.msgIndex?.toString() ?? "unknown",
		// 		});
		// 	}
		// } else {
		// 	// Warn if actor not found during cleanup
		// 	logger().warn({
		// 		msg: "websocket but actor not found for hibernatable cleanup",
		// 		actorId,
		// 		requestId,
		// 		hasHandler: !!actorHandler,
		// 		hasActor: !!actorHandler?.actor,
		// 	});
		// }
		//
		// // Also remove from ack queue
		// this.#hibernatableWebSocketAckQueue.delete(requestId);
	}

	onAfterPersistConn(conn: AnyConn) {
		// TODO:
		// this.#runner.sendHibernatableWebSocketMessageAck(
		// 	requestId,
		// 	messageIndex,
		// );
		// this.#hibernatableWebSocketAckQueue.delete(conn.id);
	}
}
