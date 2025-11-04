import type {
	ActorConfig as EngineActorConfig,
	RunnerConfig as EngineRunnerConfig,
	HibernationConfig,
} from "@rivetkit/engine-runner";
import { Runner } from "@rivetkit/engine-runner";
import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import { streamSSE } from "hono/streaming";
import { WSContext } from "hono/ws";
import invariant from "invariant";
import { lookupInRegistry } from "@/actor/definition";
import { PERSIST_SYMBOL } from "@/actor/instance";
import { deserializeActorKey } from "@/actor/keys";
import { EncodingSchema } from "@/actor/protocol/serde";
import { type ActorRouter, createActorRouter } from "@/actor/router";
import {
	handleRawWebSocketHandler,
	handleWebSocketConnect,
	truncateRawWebSocketPathPrefix,
} from "@/actor/router-endpoints";
import type { Client } from "@/client/client";
import {
	PATH_CONNECT_WEBSOCKET,
	PATH_RAW_WEBSOCKET_PREFIX,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_TOKEN,
} from "@/common/actor-router-consts";
import type { UpgradeWebSocketArgs } from "@/common/inline-websocket-adapter2";
import { getLogger } from "@/common/log";
import type {
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import {
	type ActorDriver,
	type AnyActorInstance,
	type ManagerDriver,
	serializeEmptyPersistData,
} from "@/driver-helpers/mod";
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
import { KEYS } from "./kv";
import { logger } from "./log";

interface ActorHandler {
	actor?: AnyActorInstance;
	actorStartPromise?: ReturnType<typeof promiseWithResolvers<void>>;
	persistedData?: Uint8Array;
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
		const token = runConfig.token ?? runConfig.token;
		if (token && runConfig.inspector && runConfig.inspector.enabled) {
			runConfig.inspector.token = () => token;
		}

		this.#actorRouter = createActorRouter(
			runConfig,
			this,
			registryConfig.test.enabled,
		);

		// Create runner configuration
		let hasDisconnected = false;
		const engineRunnerConfig: EngineRunnerConfig = {
			version: this.#version,
			endpoint: getEndpoint(runConfig),
			token,
			namespace: runConfig.namespace ?? runConfig.namespace,
			totalSlots: runConfig.totalSlots ?? runConfig.totalSlots,
			runnerName: runConfig.runnerName ?? runConfig.runnerName,
			runnerKey: runConfig.runnerKey,
			metadata: {
				inspectorToken: this.#runConfig.inspector.token(),
			},
			prepopulateActorNames: buildActorNames(registryConfig),
			onConnected: () => {
				if (hasDisconnected) {
					logger().info({
						msg: "runner reconnected",
						namespace: this.#runConfig.namespace,
						runnerName: this.#runConfig.runnerName,
					});
				} else {
					logger().debug({
						msg: "runner connected",
						namespace: this.#runConfig.namespace,
						runnerName: this.#runConfig.runnerName,
					});
				}

				this.#runnerStarted.resolve(undefined);
			},
			onDisconnected: (code, reason) => {
				logger().warn({
					msg: "runner disconnected",
					namespace: this.#runConfig.namespace,
					runnerName: this.#runConfig.runnerName,
					code,
					reason,
				});
				hasDisconnected = true;
			},
			onShutdown: () => {
				this.#runnerStopped.resolve(undefined);
			},
			fetch: this.#runnerFetch.bind(this),
			websocket: this.#runnerWebSocket.bind(this),
			onActorStart: this.#runnerOnActorStart.bind(this),
			onActorStop: this.#runnerOnActorStop.bind(this),
			logger: getLogger("engine-runner"),
			getActorHibernationConfig: (
				actorId: string,
				requestId: ArrayBuffer,
				request: Request,
			): HibernationConfig => {
				const url = new URL(request.url);
				const path = url.pathname;

				// Get actor instance from runner to access actor name
				const actorInstance = this.#runner.getActor(actorId);
				if (!actorInstance) {
					logger().warn({
						msg: "actor not found in getActorHibernationConfig",
						actorId,
					});
					return { enabled: false, lastMsgIndex: undefined };
				}

				// Load actor handler to access persisted data
				const handler = this.#actors.get(actorId);
				if (!handler) {
					logger().warn({
						msg: "actor handler not found in getActorHibernationConfig",
						actorId,
					});
					return { enabled: false, lastMsgIndex: undefined };
				}
				if (!handler.actor) {
					logger().warn({
						msg: "actor not found in getActorHibernationConfig",
						actorId,
					});
					return { enabled: false, lastMsgIndex: undefined };
				}

				// Check for existing WS
				const existingWs = handler.actor[
					PERSIST_SYMBOL
				].hibernatableWebSocket.find((ws) =>
					arrayBuffersEqual(ws.requestId, requestId),
				);

				// Determine configuration for new WS
				let hibernationConfig: HibernationConfig;
				if (existingWs) {
					hibernationConfig = {
						enabled: true,
						lastMsgIndex: Number(existingWs.msgIndex),
					};
				} else {
					if (path === PATH_CONNECT_WEBSOCKET) {
						hibernationConfig = {
							enabled: true,
							lastMsgIndex: undefined,
						};
					} else if (path.startsWith(PATH_RAW_WEBSOCKET_PREFIX)) {
						// Find actor config
						const definition = lookupInRegistry(
							this.#registryConfig,
							actorInstance.config.name,
						);

						// Check if can hibernate
						const canHibernatWebSocket =
							definition.config.options?.canHibernatWebSocket;
						if (canHibernatWebSocket === true) {
							hibernationConfig = {
								enabled: true,
								lastMsgIndex: undefined,
							};
						} else if (typeof canHibernatWebSocket === "function") {
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
									canHibernatWebSocket(truncatedRequest);
								hibernationConfig = {
									enabled: canHibernate,
									lastMsgIndex: undefined,
								};
							} catch (error) {
								logger().error({
									msg: "error calling canHibernatWebSocket",
									error,
								});
								hibernationConfig = {
									enabled: false,
									lastMsgIndex: undefined,
								};
							}
						} else {
							hibernationConfig = {
								enabled: false,
								lastMsgIndex: undefined,
							};
						}
					} else {
						logger().warn({
							msg: "unexpected path for getActorHibernationConfig",
							path,
						});
						hibernationConfig = {
							enabled: false,
							lastMsgIndex: undefined,
						};
					}
				}

				// Save hibernatable WebSocket
				handler.actor[PERSIST_SYMBOL].hibernatableWebSocket.push({
					requestId,
					lastSeenTimestamp: BigInt(Date.now()),
					msgIndex: -1n,
				});

				return hibernationConfig;
			},
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

	async #loadActorHandler(actorId: string): Promise<ActorHandler> {
		// Check if actor is already loaded
		const handler = this.#actors.get(actorId);
		if (!handler)
			throw new Error(`Actor handler does not exist ${actorId}`);
		if (handler.actorStartPromise) await handler.actorStartPromise.promise;
		if (!handler.actor) throw new Error("Actor should be loaded");
		return handler;
	}

	async loadActor(actorId: string): Promise<AnyActorInstance> {
		const handler = await this.#loadActorHandler(actorId);
		if (!handler.actor) throw new Error(`Actor ${actorId} failed to load`);
		return handler.actor;
	}

	getContext(actorId: string): DriverContext {
		return {};
	}

	async readPersistedData(actorId: string): Promise<Uint8Array | undefined> {
		const handler = this.#actors.get(actorId);
		if (!handler) throw new Error(`Actor ${actorId} not loaded`);

		// This was loaded during actor startup
		return handler.persistedData;
	}

	async writePersistedData(actorId: string, data: Uint8Array): Promise<void> {
		const handler = this.#actors.get(actorId);
		if (!handler) throw new Error(`Actor ${actorId} not loaded`);

		handler.persistedData = data;

		await this.#runner.kvPut(actorId, [[KEYS.PERSIST_DATA, data]]);
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
			actor._onAlarm();
			this.#alarmTimeout = undefined;
		}, delay);

		// TODO: This call may not be needed on ActorInstance.start, but it does help ensure that the local state is synced with the alarm state
		// Set alarm on Rivet
		//
		// This does not call an "alarm" event like Durable Objects.
		// Instead, it just wakes the actor on the alarm (if not
		// already awake).
		//
		// _onAlarm is automatically called on `ActorInstance.start` when waking
		// again.
		this.#runner.setAlarm(actor.id, timestamp);
	}

	async getDatabase(_actorId: string): Promise<unknown | undefined> {
		return undefined;
	}

	// Runner lifecycle callbacks
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
				persistedData: undefined,
			};
			this.#actors.set(actorId, handler);

			// Load persisted data from storage
			const [persistedValue] = await this.#runner.kvGet(actorId, [
				KEYS.PERSIST_DATA,
			]);

			handler.persistedData =
				persistedValue !== null
					? persistedValue
					: serializeEmptyPersistData(input);
		}

		const name = actorConfig.name as string;
		invariant(actorConfig.key, "actor should have a key");
		const key = deserializeActorKey(actorConfig.key);

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

		const handler = this.#actors.get(actorId);
		if (handler?.actor) {
			try {
				await handler.actor._onStop();
			} catch (err) {
				logger().error({
					msg: "error in _onStop, proceeding with removing actor",
					err: stringifyError(err),
				});
			}
			this.#actors.delete(actorId);
		}

		logger().debug({ msg: "runner actor stopped", actorId });
	}

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

	async #runnerWebSocket(
		_runner: Runner,
		actorId: string,
		websocketRaw: any,
		requestIdBuf: ArrayBuffer,
		request: Request,
	): Promise<void> {
		const websocket = websocketRaw as UniversalWebSocket;
		const requestId = idToStr(requestIdBuf);

		logger().debug({ msg: "runner websocket", actorId, url: request.url });

		const url = new URL(request.url);

		// Parse configuration from Sec-WebSocket-Protocol header (optional for path-based routing)
		const protocols = request.headers.get("sec-websocket-protocol");

		let encodingRaw: string | undefined;
		let connParamsRaw: string | undefined;

		if (protocols) {
			const protocolList = protocols.split(",").map((p) => p.trim());
			for (const protocol of protocolList) {
				if (protocol.startsWith(WS_PROTOCOL_ENCODING)) {
					encodingRaw = protocol.substring(
						WS_PROTOCOL_ENCODING.length,
					);
				} else if (protocol.startsWith(WS_PROTOCOL_CONN_PARAMS)) {
					connParamsRaw = decodeURIComponent(
						protocol.substring(WS_PROTOCOL_CONN_PARAMS.length),
					);
				}
			}
		}

		const encoding = EncodingSchema.parse(encodingRaw);
		const connParams = connParamsRaw
			? JSON.parse(connParamsRaw)
			: undefined;

		// Fetch WS handler
		//
		// We store the promise since we need to add WebSocket event listeners immediately that will wait for the promise to resolve
		let wsHandlerPromise: Promise<UpgradeWebSocketArgs>;
		if (url.pathname === PATH_CONNECT_WEBSOCKET) {
			wsHandlerPromise = handleWebSocketConnect(
				request,
				this.#runConfig,
				this,
				actorId,
				encoding,
				connParams,
				requestId,
				// Extract connId and connToken from protocols if needed
				undefined,
				undefined,
			);
		} else if (url.pathname.startsWith(PATH_RAW_WEBSOCKET_PREFIX)) {
			wsHandlerPromise = handleRawWebSocketHandler(
				request,
				url.pathname + url.search,
				this,
				actorId,
			);
		} else {
			throw new Error(`Unreachable path: ${url.pathname}`);
		}

		// TODO: Add close

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
			wsHandlerPromise.then((x) => x.onMessage?.(event, wsContext));

			invariant(event.rivetRequestId, "missing rivetRequestId");
			invariant(event.rivetMessageIndex, "missing rivetMessageIndex");
			this.#runner.sendWebsocketMessageAck(
				event.rivetRequestId,
				event.rivetMessageIndex,
			);
		});

		websocket.addEventListener("close", (event) => {
			wsHandlerPromise.then((x) => x.onClose?.(event, wsContext));
		});

		websocket.addEventListener("error", (event) => {
			wsHandlerPromise.then((x) => x.onError?.(event, wsContext));
		});
	}

	startSleep(actorId: string) {
		this.#runner.sleepActor(actorId);
	}

	async shutdownRunner(immediate: boolean): Promise<void> {
		logger().info({ msg: "stopping engine actor driver" });
		await this.#runner.shutdown(immediate);
	}

	async serverlessHandleStart(c: HonoContext): Promise<Response> {
		return streamSSE(c, async (stream) => {
			// NOTE: onAbort does not work reliably
			stream.onAbort(() => {});
			c.req.raw.signal.addEventListener("abort", () => {
				logger().debug("SSE aborted, shutting down runner");
				this.shutdownRunner(true);
			});

			await this.#runnerStarted.promise;

			// Runner id should be set if the runner started
			const payload = this.#runner.getServerlessInitPacket();
			invariant(payload, "runnerId not set");
			await stream.writeSSE({ data: payload });

			await this.#runnerStopped.promise;
		});
	}
}
