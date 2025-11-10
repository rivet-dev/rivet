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
import { logger } from "./log";

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

	// WebSocket message acknowledgment debouncing for hibernatable websockets
	#hibernatableWebSocketAckQueue: Map<
		string,
		{ requestIdBuf: ArrayBuffer; messageIndex: number }
	> = new Map();
	#wsAckFlushInterval?: NodeJS.Timeout;

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
				const hibernatableArray =
					handler.actor.persist.hibernatableConns;
				logger().debug({
					msg: "checking hibernatable websockets",
					requestId: idToStr(requestId),
					existingHibernatableWebSockets: hibernatableArray.length,
					actorId,
				});

				const existingWs = hibernatableArray.find((conn) =>
					arrayBuffersEqual(conn.hibernatableRequestId, requestId),
				);

				// Determine configuration for new WS
				let hibernationConfig: HibernationConfig;
				if (existingWs) {
					// Convert msgIndex to number, treating -1 as undefined (no messages processed yet)
					const lastMsgIndex =
						existingWs.msgIndex >= 0n
							? Number(existingWs.msgIndex)
							: undefined;
					logger().debug({
						msg: "found existing hibernatable websocket",
						requestId: idToStr(requestId),
						lastMsgIndex: lastMsgIndex ?? -1,
					});
					hibernationConfig = {
						enabled: true,
						lastMsgIndex,
					};
				} else {
					logger().debug({
						msg: "no existing hibernatable websocket found",
						requestId: idToStr(requestId),
					});
					if (path === PATH_CONNECT) {
						hibernationConfig = {
							enabled: true,
							lastMsgIndex: undefined,
						};
					} else if (path.startsWith(PATH_WEBSOCKET_PREFIX)) {
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

				// Save or update hibernatable WebSocket
				if (existingWs) {
					logger().debug({
						msg: "updated existing hibernatable websocket timestamp",
						requestId: idToStr(requestId),
						currentMsgIndex: existingWs.msgIndex,
					});
					existingWs.lastSeenTimestamp = Date.now();
				} else if (path === PATH_CONNECT) {
					// For new hibernatable connections, we'll create a placeholder entry
					// The actual connection data will be populated when the connection is created
					logger().debug({
						msg: "will create hibernatable conn when connection is created",
						requestId: idToStr(requestId),
					});
					// Note: The actual hibernatable connection is created in connection-manager.ts
					// when createConn is called with a hibernatable requestId
				}

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

		// Start WebSocket ack flush interval
		//
		// Decreasing this reduces the amount of buffered messages on the
		// gateway
		//
		// Gateway timeout configured to 30s
		// https://github.com/rivet-dev/rivet/blob/222dae87e3efccaffa2b503de40ecf8afd4e31eb/engine/packages/pegboard-gateway/src/shared_state.rs#L17
		this.#wsAckFlushInterval = setInterval(
			() => this.#flushHibernatableWebSocketAcks(),
			1000,
		);
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

	#flushHibernatableWebSocketAcks(): void {
		if (this.#hibernatableWebSocketAckQueue.size === 0) return;

		for (const {
			requestIdBuf: requestId,
			messageIndex: index,
		} of this.#hibernatableWebSocketAckQueue.values()) {
			this.#runner.sendWebsocketMessageAck(requestId, index);
		}

		this.#hibernatableWebSocketAckQueue.clear();
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

	// Batch KV operations
	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		logger().debug({
			msg: "batch writing KV entries",
			actorId,
			entryCount: entries.length,
		});

		await this.#runner.kvPut(actorId, entries);
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		logger().debug({
			msg: "batch reading KV entries",
			actorId,
			keyCount: keys.length,
		});

		return await this.#runner.kvGet(actorId, keys);
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		logger().debug({
			msg: "batch deleting KV entries",
			actorId,
			keyCount: keys.length,
		});

		await this.#runner.kvDelete(actorId, keys);
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		logger().debug({
			msg: "listing KV entries with prefix",
			actorId,
			prefixLength: prefix.length,
		});

		return await this.#runner.kvListPrefix(actorId, prefix);
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
			};
			this.#actors.set(actorId, handler);
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
				await handler.actor.onStop();
			} catch (err) {
				logger().error({
					msg: "error in onStop, proceeding with removing actor",
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
			invariant(event.rivetRequestId, "missing rivetRequestId");
			invariant(event.rivetMessageIndex, "missing rivetMessageIndex");

			// Handle hibernatable WebSockets:
			// - Check for out of sequence messages
			// - Save msgIndex for WS restoration
			// - Queue WS acks
			const actorHandler = this.#actors.get(actorId);
			if (actorHandler?.actor) {
				const hibernatableWs =
					actorHandler.actor.persist.hibernatableConns.find(
						(conn: any) =>
							arrayBuffersEqual(
								conn.hibernatableRequestId,
								requestIdBuf,
							),
					);

				if (hibernatableWs) {
					// Track msgIndex for sending acks
					const currentEntry =
						this.#hibernatableWebSocketAckQueue.get(requestId);
					if (currentEntry) {
						const previousIndex = currentEntry.messageIndex;

						// Check for out-of-sequence messages
						if (event.rivetMessageIndex !== previousIndex + 1) {
							let closeReason: string;
							let sequenceType: string;

							if (event.rivetMessageIndex < previousIndex) {
								closeReason = "ws.message_index_regressed";
								sequenceType = "regressed";
							} else if (
								event.rivetMessageIndex === previousIndex
							) {
								closeReason = "ws.message_index_duplicate";
								sequenceType = "duplicate";
							} else {
								closeReason = "ws.message_index_skip";
								sequenceType = "gap/skipped";
							}

							logger().warn({
								msg: "hibernatable websocket message index out of sequence, closing connection",
								requestId,
								actorId,
								previousIndex,
								expectedIndex: previousIndex + 1,
								receivedIndex: event.rivetMessageIndex,
								sequenceType,
								closeReason,
								gap:
									event.rivetMessageIndex > previousIndex
										? event.rivetMessageIndex -
											previousIndex -
											1
										: 0,
							});

							// Close the WebSocket and skip processing
							wsContext.close(1008, closeReason);
							return;
						}

						// Update to the next index
						currentEntry.messageIndex = event.rivetMessageIndex;
					} else {
						this.#hibernatableWebSocketAckQueue.set(requestId, {
							requestIdBuf,
							messageIndex: event.rivetMessageIndex,
						});
					}

					// Update msgIndex for next WebSocket open msgIndex restoration
					const oldMsgIndex = hibernatableWs.msgIndex;
					hibernatableWs.msgIndex = event.rivetMessageIndex;
					hibernatableWs.lastSeenTimestamp = Date.now();

					logger().debug({
						msg: "updated hibernatable websocket msgIndex in engine driver",
						requestId,
						oldMsgIndex: oldMsgIndex.toString(),
						newMsgIndex: event.rivetMessageIndex,
						actorId,
					});
				}
			} else {
				// Warn if we receive a message for a hibernatable websocket but can't find the actor
				logger().warn({
					msg: "received websocket message but actor not found for hibernatable tracking",
					actorId,
					requestId,
					messageIndex: event.rivetMessageIndex,
					hasHandler: !!actorHandler,
					hasActor: !!actorHandler?.actor,
				});
			}

			// Process the message after all hibernation logic and validation in case the message is out of order
			wsHandlerPromise.then((x) => x.onMessage?.(event, wsContext));
		});

		websocket.addEventListener("close", (event) => {
			// Flush any pending acks before closing
			this.#flushHibernatableWebSocketAcks();

			// Clean up hibernatable WebSocket
			this.#cleanupHibernatableWebSocket(
				actorId,
				requestIdBuf,
				requestId,
				"close",
				event,
			);

			wsHandlerPromise.then((x) => x.onClose?.(event, wsContext));
		});

		websocket.addEventListener("error", (event) => {
			// Clean up hibernatable WebSocket on error
			this.#cleanupHibernatableWebSocket(
				actorId,
				requestIdBuf,
				requestId,
				"error",
				event,
			);

			wsHandlerPromise.then((x) => x.onError?.(event, wsContext));
		});
	}

	/**
	 * Helper method to clean up hibernatable WebSocket entries
	 * Eliminates duplication between close and error handlers
	 */
	#cleanupHibernatableWebSocket(
		actorId: string,
		requestIdBuf: ArrayBuffer,
		requestId: string,
		eventType: "close" | "error",
		event?: any,
	) {
		const actorHandler = this.#actors.get(actorId);
		if (actorHandler?.actor) {
			const hibernatableArray =
				actorHandler.actor.persist.hibernatableConns;
			const wsIndex = hibernatableArray.findIndex((conn: any) =>
				arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
			);

			if (wsIndex !== -1) {
				const removed = hibernatableArray.splice(wsIndex, 1);
				const logData: any = {
					msg: `removed hibernatable websocket on ${eventType}`,
					requestId,
					actorId,
					removedMsgIndex:
						removed[0]?.msgIndex?.toString() ?? "unknown",
				};
				// Add error context if this is an error event
				if (eventType === "error" && event) {
					logData.error = event;
				}
				logger().debug(logData);
			}
		} else {
			// Warn if actor not found during cleanup
			const warnData: any = {
				msg: `websocket ${eventType === "close" ? "closed" : "error"} but actor not found for hibernatable cleanup`,
				actorId,
				requestId,
				hasHandler: !!actorHandler,
				hasActor: !!actorHandler?.actor,
			};
			// Add error context if this is an error event
			if (eventType === "error" && event) {
				warnData.error = event;
			}
			logger().warn(warnData);
		}

		// Also remove from ack queue
		this.#hibernatableWebSocketAckQueue.delete(requestId);
	}

	startSleep(actorId: string) {
		this.#runner.sleepActor(actorId);
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
					handler.actor.onStop().catch((err) => {
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

		// Clear the ack flush interval
		if (this.#wsAckFlushInterval) {
			clearInterval(this.#wsAckFlushInterval);
			this.#wsAckFlushInterval = undefined;
		}

		// Flush any remaining acks
		this.#flushHibernatableWebSocketAcks();

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

	getExtraActorLogParams(): Record<string, string> {
		return { runnerId: this.#runner.runnerId ?? "-" };
	}
}
