import * as protocol from "@rivetkit/engine-runner-protocol";
import type { Logger } from "pino";
import type WebSocket from "ws";
import { type ActorConfig, RunnerActor } from "./actor";
import { logger, setLogger } from "./log.js";
import { stringifyToClient, stringifyToServer } from "./stringify";
import { type HibernatingWebSocketMetadata, Tunnel } from "./tunnel";
import {
	calculateBackoff,
	parseWebSocketCloseReason,
	stringifyError,
	unreachable,
} from "./utils";
import { importWebSocket } from "./websocket.js";

export type { HibernatingWebSocketMetadata };
export { RunnerActor, type ActorConfig };
export { idToStr } from "./utils";

const KV_EXPIRE: number = 30_000;
const PROTOCOL_VERSION: number = 5;

/** Warn once the backlog significantly exceeds the server's ack batch size. */
const EVENT_BACKLOG_WARN_THRESHOLD = 10_000;
const SIGNAL_HANDLERS: (() => void | Promise<void>)[] = [];

export class RunnerShutdownError extends Error {
	constructor() {
		super("Runner shut down");
	}
}

export interface RunnerConfig {
	logger?: Logger;
	version: number;
	endpoint: string;
	token?: string;
	pegboardEndpoint?: string;
	pegboardRelayEndpoint?: string;
	namespace: string;
	totalSlots: number;
	runnerName: string;
	runnerKey: string;
	prepopulateActorNames: Record<string, { metadata: Record<string, any> }>;
	metadata?: Record<string, any>;
	onConnected: () => void;
	onDisconnected: (code: number, reason: string) => void;
	onShutdown: () => void;

	/** Called when receiving a network request. */
	fetch: (
		runner: Runner,
		actorId: string,
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		request: Request,
	) => Promise<Response>;

	/**
	 * Called when receiving a WebSocket connection.
	 *
	 * All event listeners must be added synchronously inside this function or
	 * else events may be missed. The open event will fire immediately after
	 * this function finishes.
	 *
	 * Any errors thrown here will disconnect the WebSocket immediately.
	 *
	 * While `path` and `headers` are partially redundant to the data in the
	 * `Request`, they may vary slightly from the actual content of `Request`.
	 * Prefer to persist the `path` and `headers` properties instead of the
	 * `Request` itself.
	 *
	 * ## Hibernating Web Sockets
	 *
	 * ### Implementation Requirements
	 *
	 * **Requirement 1: Persist HWS Immediately**
	 *
	 * This is responsible for persisting hibernatable WebSockets immediately
	 * (do not wait for open event). It is not time sensitive to flush the
	 * connection state. If this fails to persist the HWS, the client's
	 * WebSocket will be disconnected on next wake in the call to
	 * `Tunnel::restoreHibernatingRequests` since the connection entry will not
	 * exist.
	 *
	 * **Requirement 2: Persist Message Index On `message`**
	 *
	 * In the `message` event listener, this handler must persist the message
	 * index from the event. The request ID is available at
	 * `event.rivetRequestId` and message index at `event.rivetMessageIndex`.
	 *
	 * The message index should not be flushed immediately. Instead, this
	 * should:
	 *
	 * - Debounce calls to persist the message index
	 * - After each persist, call
	 *   `Runner::sendHibernatableWebSocketMessageAck` to acknowledge the
	 *   message
	 *
	 * This mechanism allows us to buffer messages on the gateway so we can
	 * batch-persist events on our end on a given interval.
	 *
	 * If this fails to persist, then the gateway will replay unacked
	 * messages when the actor starts again.
	 *
	 * **Requirement 3: Remove HWS From Storage On `close`**
	 *
	 * This handler should add an event listener for `close` to remove the
	 * connection from storage.
	 *
	 * If the connection remove fails to persist, the close event will be
	 * called again on the next actor start in
	 * `Tunnel::restoreHibernatingRequests` since there will be no request for
	 * the given connection.
	 *
	 * ### Restoring Connections
	 *
	 * The user of this library is responsible for:
	 * 1. Loading all persisted hibernatable WebSocket metadata for an actor
	 * 2. Calling `Runner::restoreHibernatingRequests` with this metadata at
	 *    the end of `onActorStart`
	 *
	 * `restoreHibernatingRequests` will restore all connections and attach
	 * the appropriate event listeners.
	 *
	 * ### No Open Event On Restoration
	 *
	 * When restoring a HWS, the open event will not be called again. It will
	 * go straight to the message or close event.
	 */
	websocket: (
		runner: Runner,
		actorId: string,
		ws: any,
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		request: Request,
		path: string,
		headers: Record<string, string>,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
	) => Promise<void>;

	hibernatableWebSocket: {
		/**
		 * Determines if a WebSocket can continue to live while an actor goes to
		 * sleep.
		 */
		canHibernate: (
			actorId: string,
			gatewayId: ArrayBuffer,
			requestId: ArrayBuffer,
			request: Request,
		) => boolean;
	};

	/**
	 * Called when an actor starts.
	 *
	 * This callback is responsible for:
	 * 1. Initializing the actor instance
	 * 2. Loading all persisted hibernatable WebSocket metadata for this actor
	 * 3. Calling `Runner::restoreHibernatingRequests` with the loaded metadata
	 *    to restore hibernatable WebSocket connections
	 *
	 * The actor should not be marked as "ready" until after
	 * `restoreHibernatingRequests` completes to ensure all hibernatable
	 * connections are fully restored before the actor processes new requests.
	 */
	onActorStart: (
		actorId: string,
		generation: number,
		config: ActorConfig,
	) => Promise<void>;

	onActorStop: (actorId: string, generation: number) => Promise<void>;
	noAutoShutdown?: boolean;
}

export interface KvListOptions {
	reverse?: boolean;
	limit?: number;
}

interface KvRequestEntry {
	actorId: string;
	data: protocol.KvRequestData;
	resolve: (value: any) => void;
	reject: (error: unknown) => void;
	sent: boolean;
	timestamp: number;
}

export class Runner {
	#config: RunnerConfig;

	get config(): RunnerConfig {
		return this.#config;
	}

	#actors: Map<string, RunnerActor> = new Map();

	// WebSocket
	#pegboardWebSocket?: WebSocket;
	runnerId?: string;
	#started: boolean = false;
	#shutdown: boolean = false;
	#reconnectAttempt: number = 0;
	#reconnectTimeout?: NodeJS.Timeout;

	// Protocol metadata
	#protocolMetadata?: protocol.ProtocolMetadata;

	// Runner lost threshold management
	#runnerLostTimeout?: NodeJS.Timeout;

	// Event storage for resending
	#eventBacklogWarned: boolean = false;

	// Command acknowledgment
	#ackInterval?: NodeJS.Timeout;

	// KV operations
	#nextKvRequestId: number = 0;
	#kvRequests: Map<number, KvRequestEntry> = new Map();
	#kvCleanupInterval?: NodeJS.Timeout;

	// Tunnel for HTTP/WebSocket forwarding
	#tunnel: Tunnel | undefined;

	// Cached child logger with runner-specific attributes
	#logCached?: Logger;

	get log(): Logger | undefined {
		if (this.#logCached) return this.#logCached;

		const l = logger();
		if (l) {
			// If has connected, create child logger with relevant metadata
			//
			// Otherwise, return default logger
			if (this.runnerId) {
				this.#logCached = l.child({
					runnerId: this.runnerId,
				});
				return this.#logCached;
			} else {
				return l;
			}
		}

		return undefined;
	}

	constructor(config: RunnerConfig) {
		this.#config = config;
		if (this.#config.logger) setLogger(this.#config.logger);

		// Start cleaning up old unsent KV requests every 15 seconds
		this.#kvCleanupInterval = setInterval(() => {
			try {
				this.#cleanupOldKvRequests();
			} catch (err) {
				this.log?.error({
					msg: "error cleaning up kv requests",
					error: stringifyError(err),
				});
			}
		}, 15000); // Run every 15 seconds
	}

	// MARK: Manage actors
	sleepActor(actorId: string, generation?: number) {
		const actor = this.getActor(actorId, generation);
		if (!actor) return;

		// Keep the actor instance in memory during sleep
		this.#sendActorIntent(actorId, actor.generation, "sleep");

		// NOTE: We do NOT remove the actor from this.#actors here
		// The server will send a StopActor command if it wants to fully stop
	}

	async stopActor(actorId: string, generation?: number) {
		const actor = this.getActor(actorId, generation);
		if (!actor) return;

		this.#sendActorIntent(actorId, actor.generation, "stop");

		// NOTE: We do NOT remove the actor from this.#actors here
		// The server will send a StopActor command if it wants to fully stop
	}

	async forceStopActor(actorId: string, generation?: number) {
		this.log?.debug({
			msg: "force stopping actor",
			actorId,
		});

		const actor = this.getActor(actorId, generation);
		if (!actor) return;

		// If onActorStop times out, Pegboard will handle this timeout with ACTOR_STOP_THRESHOLD_DURATION_MS
		//
		// If we receive a request while onActorStop is running, a Service
		// Unavailable error will be returned to Guard and the request will be
		// retried
		try {
			await this.#config.onActorStop(actorId, actor.generation);
		} catch (err) {
			console.error(`Error in onActorStop for actor ${actorId}:`, err);
		}

		// Close requests after onActorStop so you can send messages over the tunnel
		this.#tunnel?.closeActiveRequests(actor);

		this.#sendActorStateUpdate(actorId, actor.generation, "stopped");

		// Remove actor after stopping in order to ensure that we can still
		// call actions on the runner
		this.#removeActor(actorId, generation);
	}

	#handleLost() {
		this.log?.info({
			msg: "stopping all actors due to runner lost threshold",
		});

		// Remove all remaining kv requests
		for (const [_, request] of this.#kvRequests.entries()) {
			request.reject(new RunnerShutdownError());
		}

		this.#kvRequests.clear();

		this.#stopAllActors();
	}

	#stopAllActors() {
		const actorIds = Array.from(this.#actors.keys());
		for (const actorId of actorIds) {
			this.forceStopActor(actorId).catch((err) => {
				this.log?.error({
					msg: "error stopping actor",
					actorId,
					error: stringifyError(err),
				});
			});
		}
	}

	getActor(actorId: string, generation?: number): RunnerActor | undefined {
		const actor = this.#actors.get(actorId);
		if (!actor) {
			this.log?.warn({
				msg: "actor not found",
				actorId,
			});
			return undefined;
		}
		if (generation !== undefined && actor.generation !== generation) {
			this.log?.warn({
				msg: "actor generation mismatch",
				actorId,
				generation,
			});
			return undefined;
		}

		return actor;
	}

	async getAndWaitForActor(
		actorId: string,
		generation?: number,
	): Promise<RunnerActor | undefined> {
		const actor = this.getActor(actorId, generation);
		if (!actor) return;
		await actor.actorStartPromise.promise;
		return actor;
	}

	hasActor(actorId: string, generation?: number): boolean {
		const actor = this.#actors.get(actorId);

		return (
			!!actor &&
			(generation === undefined || actor.generation === generation)
		);
	}

	get actors() {
		return this.#actors;
	}

	// IMPORTANT: Make sure to call stopActiveRequests if calling #removeActor
	#removeActor(
		actorId: string,
		generation?: number,
	): RunnerActor | undefined {
		const actor = this.#actors.get(actorId);
		if (!actor) {
			this.log?.error({
				msg: "actor not found for removal",
				actorId,
			});
			return undefined;
		}
		if (generation !== undefined && actor.generation !== generation) {
			this.log?.error({
				msg: "actor generation mismatch",
				actorId,
				generation,
			});
			return undefined;
		}

		this.#actors.delete(actorId);

		this.log?.info({
			msg: "removed actor",
			actorId,
			actors: this.#actors.size,
		});

		return actor;
	}

	// MARK: Start
	async start() {
		if (this.#started) throw new Error("Cannot call runner.start twice");
		this.#started = true;

		this.log?.info({ msg: "starting runner" });

		this.#tunnel = new Tunnel(this);
		this.#tunnel.start();

		try {
			await this.#openPegboardWebSocket();
		} catch (error) {
			this.#started = false;
			throw error;
		}

		if (!this.#config.noAutoShutdown) {
			if (!SIGNAL_HANDLERS.length) {
				process.on("SIGTERM", async () => {
					this.log?.debug("received SIGTERM");

					for (const handler of SIGNAL_HANDLERS) {
						await handler();
					}

					// TODO: Add back
					// process.exit(0);
				});
				process.on("SIGINT", async () => {
					this.log?.debug("received SIGINT");

					for (const handler of SIGNAL_HANDLERS) {
						await handler();
					}

					// TODO: Add back
					// process.exit(0);
				});

				this.log?.debug({
					msg: "added SIGTERM listeners",
				});
			}

			SIGNAL_HANDLERS.push(async () => {
				const weak = new WeakRef(this);
				await weak.deref()?.shutdown(false, false);
			});
		}
	}

	// MARK: Shutdown
	async shutdown(immediate: boolean, exit: boolean = false) {
		// Prevent concurrent shutdowns
		if (this.#shutdown) {
			this.log?.debug({
				msg: "shutdown already in progress, ignoring",
			});
			return;
		}
		this.#shutdown = true;

		this.log?.info({
			msg: "starting shutdown",
			immediate,
			exit,
		});

		// Clear reconnect timeout
		if (this.#reconnectTimeout) {
			clearTimeout(this.#reconnectTimeout);
			this.#reconnectTimeout = undefined;
		}

		// Clear runner lost timeout
		if (this.#runnerLostTimeout) {
			clearTimeout(this.#runnerLostTimeout);
			this.#runnerLostTimeout = undefined;
		}

		// Clear ack interval
		if (this.#ackInterval) {
			clearInterval(this.#ackInterval);
			this.#ackInterval = undefined;
		}

		// Clear KV cleanup interval
		if (this.#kvCleanupInterval) {
			clearInterval(this.#kvCleanupInterval);
			this.#kvCleanupInterval = undefined;
		}

		// Reject all KV requests
		for (const request of this.#kvRequests.values()) {
			request.reject(
				new Error("WebSocket connection closed during shutdown"),
			);
		}
		this.#kvRequests.clear();

		// Close WebSocket
		const pegboardWebSocket = this.getPegboardWebSocketIfReady();
		if (pegboardWebSocket) {
			if (immediate) {
				// Stop immediately
				pegboardWebSocket.close(1000, "pegboard.runner_shutdown");
			} else {
				// Wait for actors to shut down before stopping
				try {
					this.log?.info({
						msg: "sending stopping message",
						readyState: pegboardWebSocket.readyState,
					});

					// Start stopping
					//
					// The runner workflow will send StopActor commands for all
					// actors
					this.__sendToServer({
						tag: "ToServerStopping",
						val: null,
					});

					const closePromise = new Promise<void>((resolve) => {
						if (!pegboardWebSocket)
							throw new Error("missing pegboardWebSocket");

						pegboardWebSocket.addEventListener("close", (ev) => {
							this.log?.info({
								msg: "connection closed",
								code: ev.code,
								reason: ev.reason.toString(),
							});
							resolve();
						});
					});

					// Wait for all actors to stop before closing ws
					await this.#waitForActorsToStop(pegboardWebSocket);

					this.log?.info({
						msg: "closing WebSocket",
					});
					pegboardWebSocket.close(1000, "pegboard.runner_shutdown");

					await closePromise;

					this.log?.info({
						msg: "websocket shutdown completed",
					});
				} catch (error) {
					this.log?.error({
						msg: "error during websocket shutdown:",
						error,
					});
					pegboardWebSocket.close();
				}
			}
		} else {
			// This is often logged when the serverless SSE stream closes after
			// the runner has already shut down
			this.log?.debug({
				msg: "no runner WebSocket to shutdown or already closed",
				readyState: this.#pegboardWebSocket?.readyState,
			});
		}

		// Close tunnel
		if (this.#tunnel) {
			this.#tunnel.shutdown();
			this.#tunnel = undefined;
		}

		this.#config.onShutdown();

		if (exit) process.exit(0);
	}

	/**
	 * Wait for all actors to stop before proceeding with shutdown.
	 *
	 * This method polls every 100ms to check if all actors have been stopped.
	 *
	 * It will resolve early if:
	 * - All actors are stopped
	 * - The WebSocket connection is closed
	 * - The shutdown timeout is reached (120 seconds)
	 */
	async #waitForActorsToStop(ws: WebSocket): Promise<void> {
		const shutdownTimeout = 120_000; // 120 seconds
		const shutdownCheckInterval = 100; // Check every 100ms
		const progressLogInterval = 5_000; // Log progress every 5 seconds
		const shutdownStartTs = Date.now();
		let lastProgressLogTs = 0; // Ensure first log happens immediately

		return new Promise<void>((resolve) => {
			const checkActors = () => {
				const now = Date.now();
				const elapsed = now - shutdownStartTs;
				const wsIsClosed = ws.readyState === 2 || ws.readyState === 3;

				if (this.#actors.size === 0) {
					this.log?.info({
						msg: "all actors stopped",
						elapsed,
					});
					return true;
				} else if (wsIsClosed) {
					this.log?.warn({
						msg: "websocket closed before all actors stopped",
						remainingActors: this.#actors.size,
						elapsed,
					});
					return true;
				} else if (elapsed >= shutdownTimeout) {
					this.log?.warn({
						msg: "shutdown timeout reached, forcing close",
						remainingActors: this.#actors.size,
						elapsed,
					});
					return true;
				} else {
					// Log progress every 5 seconds
					if (now - lastProgressLogTs >= progressLogInterval) {
						this.log?.info({
							msg: "waiting for actors to stop",
							remainingActors: this.#actors.size,
							elapsed,
						});
						lastProgressLogTs = now;
					}
					return false;
				}
			};

			// Check immediately first
			if (checkActors()) {
				this.log?.debug({
					msg: "actors check completed immediately",
				});
				resolve();
				return;
			}

			this.log?.debug({
				msg: "starting actor wait interval",
				checkInterval: shutdownCheckInterval,
			});

			const interval = setInterval(() => {
				this.log?.debug({
					msg: "actor wait interval tick",
					actorCount: this.#actors.size,
				});
				if (checkActors()) {
					this.log?.debug({
						msg: "actors check completed, clearing interval",
					});
					clearInterval(interval);
					resolve();
				}
			}, shutdownCheckInterval);
		});
	}

	// MARK: Networking
	get pegboardEndpoint() {
		return this.#config.pegboardEndpoint || this.#config.endpoint;
	}
	get pegboardUrl() {
		const wsEndpoint = this.pegboardEndpoint
			.replace("http://", "ws://")
			.replace("https://", "wss://");

		// Ensure the endpoint ends with /runners/connect
		const baseUrl = wsEndpoint.endsWith("/")
			? wsEndpoint.slice(0, -1)
			: wsEndpoint;
		return `${baseUrl}/runners/connect?protocol_version=${PROTOCOL_VERSION}&namespace=${encodeURIComponent(this.#config.namespace)}&runner_key=${encodeURIComponent(this.#config.runnerKey)}`;
	}

	// MARK: Runner protocol
	async #openPegboardWebSocket() {
		const protocols = ["rivet"];
		if (this.config.token)
			protocols.push(`rivet_token.${this.config.token}`);

		const WS = await importWebSocket();

		// Assertion to clear previous WebSocket
		if (
			this.#pegboardWebSocket &&
			(this.#pegboardWebSocket.readyState === WS.CONNECTING ||
				this.#pegboardWebSocket.readyState === WS.OPEN)
		) {
			this.log?.error(
				"found duplicate pegboardWebSocket, closing previous",
			);
			this.#pegboardWebSocket.close(1000, "duplicate_websocket");
		}

		const ws = new WS(this.pegboardUrl, protocols) as any as WebSocket;
		this.#pegboardWebSocket = ws;

		this.log?.info({
			msg: "connecting",
			endpoint: this.pegboardEndpoint,
			namespace: this.#config.namespace,
			runnerKey: this.#config.runnerKey,
			hasToken: !!this.config.token,
		});

		ws.addEventListener("open", () => {
			if (this.#reconnectAttempt > 0) {
				this.log?.info({
					msg: "runner reconnected",
					namespace: this.#config.namespace,
					runnerName: this.#config.runnerName,
					reconnectAttempt: this.#reconnectAttempt,
				});
			} else {
				this.log?.debug({
					msg: "runner connected",
					namespace: this.#config.namespace,
					runnerName: this.#config.runnerName,
				});
			}

			// Reset reconnect attempt counter on successful connection
			this.#reconnectAttempt = 0;

			// Clear any pending reconnect timeout
			if (this.#reconnectTimeout) {
				clearTimeout(this.#reconnectTimeout);
				this.#reconnectTimeout = undefined;
			}

			// Clear any pending runner lost timeout since we're reconnecting
			if (this.#runnerLostTimeout) {
				clearTimeout(this.#runnerLostTimeout);
				this.#runnerLostTimeout = undefined;
			}

			// Send init message
			const init: protocol.ToServerInit = {
				name: this.#config.runnerName,
				version: this.#config.version,
				totalSlots: this.#config.totalSlots,
				prepopulateActorNames: new Map(
					Object.entries(this.#config.prepopulateActorNames).map(
						([name, data]) => [
							name,
							{ metadata: JSON.stringify(data.metadata) },
						],
					),
				),
				metadata: JSON.stringify(this.#config.metadata),
			};

			this.__sendToServer({
				tag: "ToServerInit",
				val: init,
			});

			// Start command acknowledgment interval (5 minutes)
			const ackInterval = 5 * 60 * 1000; // 5 minutes in milliseconds
			const ackLoop = setInterval(() => {
				try {
					if (ws.readyState === 1) {
						this.#sendCommandAcknowledgment();
					} else {
						clearInterval(ackLoop);
						this.log?.info({
							msg: "WebSocket not open, stopping ack loop",
						});
					}
				} catch (err) {
					this.log?.error({
						msg: "error in command acknowledgment loop",
						error: stringifyError(err),
					});
				}
			}, ackInterval);
			this.#ackInterval = ackLoop;
		});

		ws.addEventListener("message", async (ev) => {
			let buf: Uint8Array;
			if (ev.data instanceof Blob) {
				buf = new Uint8Array(await ev.data.arrayBuffer());
			} else if (Buffer.isBuffer(ev.data)) {
				buf = new Uint8Array(ev.data);
			} else {
				throw new Error(`expected binary data, got ${typeof ev.data}`);
			}

			// Parse message
			const message = protocol.decodeToClient(buf);
			this.log?.debug({
				msg: "received runner message",
				data: stringifyToClient(message),
			});

			// Handle message
			if (message.tag === "ToClientInit") {
				const init = message.val;

				if (this.runnerId !== init.runnerId) {
					this.runnerId = init.runnerId;

					// Clear actors if runner id changed
					this.#stopAllActors();
				}

				this.#protocolMetadata = init.metadata;

				this.log?.info({
					msg: "received init",
					protocolMetadata: this.#protocolMetadata,
				});

				// Resend pending events
				this.#processUnsentKvRequests();
				this.#resendUnacknowledgedEvents();
				this.#tunnel?.resendBufferedEvents();

				this.#config.onConnected();
			} else if (message.tag === "ToClientCommands") {
				const commands = message.val;
				this.#handleCommands(commands);
			} else if (message.tag === "ToClientAckEvents") {
				this.#handleAckEvents(message.val);
			} else if (message.tag === "ToClientKvResponse") {
				const kvResponse = message.val;
				this.#handleKvResponse(kvResponse);
			} else if (message.tag === "ToClientTunnelMessage") {
				this.#tunnel?.handleTunnelMessage(message.val).catch((err) => {
					this.log?.error({
						msg: "error handling tunnel message",
						error: stringifyError(err),
					});
				});
			} else if (message.tag === "ToClientPing") {
				this.__sendToServer({
					tag: "ToServerPong",
					val: {
						ts: message.val.ts,
					},
				});
			} else {
				unreachable(message);
			}
		});

		ws.addEventListener("error", (ev) => {
			this.log?.error({
				msg: `WebSocket error: ${stringifyError(ev.error)}`,
			});

			if (!this.#shutdown) {
				this.#startRunnerLostTimeout();

				// Attempt to reconnect if not stopped
				this.#scheduleReconnect();
			}
		});

		ws.addEventListener("close", async (ev) => {
			if (!this.#shutdown) {
				const closeError = parseWebSocketCloseReason(ev.reason);
				if (
					closeError?.group === "ws" &&
					closeError?.error === "eviction"
				) {
					this.log?.info("runner websocket evicted");

					this.#config.onDisconnected(ev.code, ev.reason);

					await this.shutdown(true);
				} else {
					this.log?.warn({
						msg: "runner disconnected",
						code: ev.code,
						reason: ev.reason.toString(),
						closeError,
					});

					this.#config.onDisconnected(ev.code, ev.reason);
				}

				// Clear ack interval on close
				if (this.#ackInterval) {
					clearInterval(this.#ackInterval);
					this.#ackInterval = undefined;
				}

				this.#startRunnerLostTimeout();

				// Attempt to reconnect if not stopped
				this.#scheduleReconnect();
			} else {
				this.log?.info("websocket closed");

				this.#config.onDisconnected(ev.code, ev.reason);
			}
		});
	}

	#startRunnerLostTimeout() {
		// Start runner lost timeout if we have a threshold and are not shutting down
		if (
			!this.#runnerLostTimeout &&
			this.#protocolMetadata &&
			this.#protocolMetadata.runnerLostThreshold > 0
		) {
			this.log?.info({
				msg: "starting runner lost timeout",
				seconds: this.#protocolMetadata.runnerLostThreshold / 1000n,
			});
			this.#runnerLostTimeout = setTimeout(() => {
				try {
					this.#handleLost();
				} catch (err) {
					this.log?.error({
						msg: "error handling runner lost",
						error: stringifyError(err),
					});
				}
			}, Number(this.#protocolMetadata.runnerLostThreshold));
		}
	}

	#handleCommands(commands: protocol.ToClientCommands) {
		this.log?.info({
			msg: "received commands",
			commandCount: commands.length,
		});

		for (const commandWrapper of commands) {
			if (commandWrapper.inner.tag === "CommandStartActor") {
				// Spawn background promise
				this.#handleCommandStartActor(commandWrapper).catch((err) => {
					this.log?.error({
						msg: "error handling start actor command",
						actorId: commandWrapper.checkpoint.actorId,
						error: stringifyError(err),
					});
				});

				// NOTE: We don't do this for CommandStopActor because the actor will be removed by that call
				// so we cant update the checkpoint
				const actor = this.getActor(
					commandWrapper.checkpoint.actorId,
					commandWrapper.checkpoint.generation,
				);
				if (actor)
					actor.lastCommandIdx = commandWrapper.checkpoint.index;
			} else if (commandWrapper.inner.tag === "CommandStopActor") {
				// Spawn background promise
				this.#handleCommandStopActor(commandWrapper).catch((err) => {
					this.log?.error({
						msg: "error handling stop actor command",
						actorId: commandWrapper.checkpoint.actorId,
						error: stringifyError(err),
					});
				});
			} else {
				unreachable(commandWrapper.inner);
			}
		}
	}

	#handleAckEvents(ack: protocol.ToClientAckEvents) {
		const originalTotalEvents = Array.from(this.#actors).reduce(
			(s, [_, actor]) => s + actor.eventHistory.length,
			0,
		);

		for (const [_, actor] of this.#actors) {
			const checkpoint = ack.lastEventCheckpoints.find(
				(x) => x.actorId == actor.actorId,
			);

			if (checkpoint) actor.handleAckEvents(checkpoint.index);
		}

		const totalEvents = Array.from(this.#actors).reduce(
			(s, [_, actor]) => s + actor.eventHistory.length,
			0,
		);
		const prunedCount = originalTotalEvents - totalEvents;

		if (prunedCount > 0) {
			this.log?.info({
				msg: "pruned acknowledged events",
				prunedCount,
			});
		}

		if (totalEvents <= EVENT_BACKLOG_WARN_THRESHOLD) {
			this.#eventBacklogWarned = false;
		}
	}

	/** Track events to send to the server in case we need to resend it on disconnect. */
	#recordEvent(eventWrapper: protocol.EventWrapper) {
		const actor = this.getActor(eventWrapper.checkpoint.actorId);
		if (!actor) return;

		actor.recordEvent(eventWrapper);

		const totalEvents = Array.from(this.#actors).reduce(
			(s, [_, actor]) => s + actor.eventHistory.length,
			0,
		);

		if (
			totalEvents > EVENT_BACKLOG_WARN_THRESHOLD &&
			!this.#eventBacklogWarned
		) {
			this.#eventBacklogWarned = true;
			this.log?.warn({
				msg: "unacknowledged event backlog exceeds threshold",
				backlogSize: totalEvents,
				threshold: EVENT_BACKLOG_WARN_THRESHOLD,
			});
		}
	}

	async #handleCommandStartActor(commandWrapper: protocol.CommandWrapper) {
		// IMPORTANT: Make sure no async code runs before inserting #actors and
		// calling addRequestToActor in order to prevent race conditions with
		// subsequence commands

		if (!this.#tunnel) throw new Error("missing tunnel on actor start");

		const startCommand = commandWrapper.inner
			.val as protocol.CommandStartActor;

		const actorId = commandWrapper.checkpoint.actorId;
		const generation = commandWrapper.checkpoint.generation;
		const config = startCommand.config;

		const actorConfig: ActorConfig = {
			name: config.name,
			key: config.key,
			createTs: config.createTs,
			input: config.input ? new Uint8Array(config.input) : null,
		};

		const instance = new RunnerActor(
			actorId,
			generation,
			actorConfig,
			startCommand.hibernatingRequests,
		);

		const existingActor = this.#actors.get(actorId);
		if (existingActor) {
			this.log?.warn({
				msg: "replacing existing actor in actors map",
				actorId,
				existingGeneration: existingActor.generation,
				newGeneration: generation,
				existingPendingRequests: existingActor.pendingRequests.length,
			});
		}

		this.#actors.set(actorId, instance);

		// NOTE: We have to populate the requestToActor map BEFORE running any
		// async code in order for incoming tunnel messages to wait for
		// instance.actorStartPromise before processing messages
		// TODO: Where is this GC'd if something fails?
		for (const hr of startCommand.hibernatingRequests) {
			this.#tunnel.addRequestToActor(hr.gatewayId, hr.requestId, actorId);
		}

		this.log?.info({
			msg: "created actor",
			actors: this.#actors.size,
			actorId,
			name: config.name,
			key: config.key,
			generation,
			hibernatingRequests: startCommand.hibernatingRequests.length,
		});

		this.#sendActorStateUpdate(actorId, generation, "running");

		try {
			// TODO: Add timeout to onActorStart
			// Call onActorStart asynchronously and handle errors
			this.log?.debug({
				msg: "calling onActorStart",
				actorId,
				generation,
			});
			await this.#config.onActorStart(actorId, generation, actorConfig);

			instance.actorStartPromise.resolve();
		} catch (err) {
			this.log?.error({
				msg: "error starting runner actor",
				actorId,
				err,
			});

			instance.actorStartPromise.reject(err);

			// TODO: Mark as crashed
			// Send stopped state update if start failed
			await this.forceStopActor(actorId, generation);
		}
	}

	async #handleCommandStopActor(commandWrapper: protocol.CommandWrapper) {
		const stopCommand = commandWrapper.inner
			.val as protocol.CommandStopActor;

		const actorId = commandWrapper.checkpoint.actorId;
		const generation = commandWrapper.checkpoint.generation;

		await this.forceStopActor(actorId, generation);
	}

	#sendActorIntent(
		actorId: string,
		generation: number,
		intentType: "sleep" | "stop",
	) {
		const actor = this.getActor(actorId, generation);
		if (!actor) return;

		let actorIntent: protocol.ActorIntent;

		if (intentType === "sleep") {
			actorIntent = { tag: "ActorIntentSleep", val: null };
		} else if (intentType === "stop") {
			actorIntent = {
				tag: "ActorIntentStop",
				val: null,
			};
		} else {
			unreachable(intentType);
		}

		const intentEvent: protocol.EventActorIntent = {
			intent: actorIntent,
		};

		const eventWrapper: protocol.EventWrapper = {
			checkpoint: {
				actorId,
				generation,
				index: actor.nextEventIdx++,
			},
			inner: {
				tag: "EventActorIntent",
				val: intentEvent,
			},
		};

		this.#recordEvent(eventWrapper);

		this.__sendToServer({
			tag: "ToServerEvents",
			val: [eventWrapper],
		});
	}

	#sendActorStateUpdate(
		actorId: string,
		generation: number,
		stateType: "running" | "stopped",
	) {
		const actor = this.getActor(actorId, generation);
		if (!actor) return;

		let actorState: protocol.ActorState;

		if (stateType === "running") {
			actorState = { tag: "ActorStateRunning", val: null };
		} else if (stateType === "stopped") {
			actorState = {
				tag: "ActorStateStopped",
				val: {
					code: protocol.StopCode.Ok,
					message: null,
				},
			};
		} else {
			unreachable(stateType);
		}

		const stateUpdateEvent: protocol.EventActorStateUpdate = {
			state: actorState,
		};

		const eventWrapper: protocol.EventWrapper = {
			checkpoint: {
				actorId,
				generation,
				index: actor.nextEventIdx++,
			},
			inner: {
				tag: "EventActorStateUpdate",
				val: stateUpdateEvent,
			},
		};

		this.#recordEvent(eventWrapper);

		this.__sendToServer({
			tag: "ToServerEvents",
			val: [eventWrapper],
		});
	}

	#sendCommandAcknowledgment() {
		const lastCommandCheckpoints = [];

		for (const [_, actor] of this.#actors) {
			if (actor.lastCommandIdx < 0) {
				// No commands received yet, nothing to acknowledge
				continue;
			}

			lastCommandCheckpoints.push({
				actorId: actor.actorId,
				generation: actor.generation,
				index: actor.lastCommandIdx,
			});
		}

		//this.#log?.log("Sending command acknowledgment", this.#lastCommandIdx);

		this.__sendToServer({
			tag: "ToServerAckCommands",
			val: {
				lastCommandCheckpoints,
			},
		});
	}

	#handleKvResponse(response: protocol.ToClientKvResponse) {
		const requestId = response.requestId;
		const request = this.#kvRequests.get(requestId);

		if (!request) {
			this.log?.error({
				msg: "received kv response for unknown request id",
				requestId,
			});
			return;
		}

		this.#kvRequests.delete(requestId);

		if (response.data.tag === "KvErrorResponse") {
			request.reject(
				new Error(response.data.val.message || "Unknown KV error"),
			);
		} else {
			request.resolve(response.data.val);
		}
	}

	#parseGetResponseSimple(
		response: protocol.KvGetResponse,
		requestedKeys: Uint8Array[],
	): (Uint8Array | null)[] {
		// Parse the response keys and values
		const responseKeys: Uint8Array[] = [];
		const responseValues: Uint8Array[] = [];

		for (const key of response.keys) {
			responseKeys.push(new Uint8Array(key));
		}

		for (const value of response.values) {
			responseValues.push(new Uint8Array(value));
		}

		// Map response back to requested key order
		const result: (Uint8Array | null)[] = [];
		for (const requestedKey of requestedKeys) {
			let found = false;
			for (let i = 0; i < responseKeys.length; i++) {
				if (this.#keysEqual(requestedKey, responseKeys[i])) {
					result.push(responseValues[i]);
					found = true;
					break;
				}
			}
			if (!found) {
				result.push(null);
			}
		}

		return result;
	}

	#keysEqual(key1: Uint8Array, key2: Uint8Array): boolean {
		if (key1.length !== key2.length) return false;
		for (let i = 0; i < key1.length; i++) {
			if (key1[i] !== key2[i]) return false;
		}
		return true;
	}

	//#parseGetResponse(response: protocol.KvGetResponse) {
	//	const keys: string[] = [];
	//	const values: Uint8Array[] = [];
	//	const metadata: { version: Uint8Array; createTs: bigint }[] = [];
	//
	//	for (const key of response.keys) {
	//		keys.push(new TextDecoder().decode(key));
	//	}
	//
	//	for (const value of response.values) {
	//		values.push(new Uint8Array(value));
	//	}
	//
	//	for (const meta of response.metadata) {
	//		metadata.push({
	//			version: new Uint8Array(meta.version),
	//			createTs: meta.createTs,
	//		});
	//	}
	//
	//	return { keys, values, metadata };
	//}

	#parseListResponseSimple(
		response: protocol.KvListResponse,
	): [Uint8Array, Uint8Array][] {
		const result: [Uint8Array, Uint8Array][] = [];

		for (let i = 0; i < response.keys.length; i++) {
			const key = response.keys[i];
			const value = response.values[i];

			if (key && value) {
				const keyBytes = new Uint8Array(key);
				const valueBytes = new Uint8Array(value);
				result.push([keyBytes, valueBytes]);
			}
		}

		return result;
	}

	//#parseListResponse(response: protocol.KvListResponse) {
	//	const keys: string[] = [];
	//	const values: Uint8Array[] = [];
	//	const metadata: { version: Uint8Array; createTs: bigint }[] = [];
	//
	//	for (const key of response.keys) {
	//		keys.push(new TextDecoder().decode(key));
	//	}
	//
	//	for (const value of response.values) {
	//		values.push(new Uint8Array(value));
	//	}
	//
	//	for (const meta of response.metadata) {
	//		metadata.push({
	//			version: new Uint8Array(meta.version),
	//			createTs: meta.createTs,
	//		});
	//	}
	//
	//	return { keys, values, metadata };
	//}

	// MARK: KV Operations
	async kvGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		const kvKeys: protocol.KvKey[] = keys.map(
			(key) =>
				key.buffer.slice(
					key.byteOffset,
					key.byteOffset + key.byteLength,
				) as ArrayBuffer,
		);

		const requestData: protocol.KvRequestData = {
			tag: "KvGetRequest",
			val: { keys: kvKeys },
		};

		const response = await this.#sendKvRequest(actorId, requestData);
		return this.#parseGetResponseSimple(response, keys);
	}

	async kvListAll(
		actorId: string,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]> {
		const requestData: protocol.KvRequestData = {
			tag: "KvListRequest",
			val: {
				query: { tag: "KvListAllQuery", val: null },
				reverse: options?.reverse || null,
				limit:
					options?.limit !== undefined ? BigInt(options.limit) : null,
			},
		};

		const response = await this.#sendKvRequest(actorId, requestData);
		return this.#parseListResponseSimple(response);
	}

	async kvListRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
		exclusive?: boolean,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]> {
		const startKey: protocol.KvKey = start.buffer.slice(
			start.byteOffset,
			start.byteOffset + start.byteLength,
		) as ArrayBuffer;
		const endKey: protocol.KvKey = end.buffer.slice(
			end.byteOffset,
			end.byteOffset + end.byteLength,
		) as ArrayBuffer;

		const requestData: protocol.KvRequestData = {
			tag: "KvListRequest",
			val: {
				query: {
					tag: "KvListRangeQuery",
					val: {
						start: startKey,
						end: endKey,
						exclusive: exclusive || false,
					},
				},
				reverse: options?.reverse || null,
				limit:
					options?.limit !== undefined ? BigInt(options.limit) : null,
			},
		};

		const response = await this.#sendKvRequest(actorId, requestData);
		return this.#parseListResponseSimple(response);
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]> {
		const prefixKey: protocol.KvKey = prefix.buffer.slice(
			prefix.byteOffset,
			prefix.byteOffset + prefix.byteLength,
		) as ArrayBuffer;

		const requestData: protocol.KvRequestData = {
			tag: "KvListRequest",
			val: {
				query: {
					tag: "KvListPrefixQuery",
					val: { key: prefixKey },
				},
				reverse: options?.reverse || null,
				limit:
					options?.limit !== undefined ? BigInt(options.limit) : null,
			},
		};

		const response = await this.#sendKvRequest(actorId, requestData);
		return this.#parseListResponseSimple(response);
	}

	async kvPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		const keys: protocol.KvKey[] = entries.map(
			([key, _value]) =>
				key.buffer.slice(
					key.byteOffset,
					key.byteOffset + key.byteLength,
				) as ArrayBuffer,
		);
		const values: protocol.KvValue[] = entries.map(
			([_key, value]) =>
				value.buffer.slice(
					value.byteOffset,
					value.byteOffset + value.byteLength,
				) as ArrayBuffer,
		);

		const requestData: protocol.KvRequestData = {
			tag: "KvPutRequest",
			val: { keys, values },
		};

		await this.#sendKvRequest(actorId, requestData);
	}

	async kvDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		const kvKeys: protocol.KvKey[] = keys.map(
			(key) =>
				key.buffer.slice(
					key.byteOffset,
					key.byteOffset + key.byteLength,
				) as ArrayBuffer,
		);

		const requestData: protocol.KvRequestData = {
			tag: "KvDeleteRequest",
			val: { keys: kvKeys },
		};

		await this.#sendKvRequest(actorId, requestData);
	}

	async kvDrop(actorId: string): Promise<void> {
		const requestData: protocol.KvRequestData = {
			tag: "KvDropRequest",
			val: null,
		};

		await this.#sendKvRequest(actorId, requestData);
	}

	// MARK: Alarm Operations
	setAlarm(actorId: string, alarmTs: number | null, generation?: number) {
		const actor = this.getActor(actorId, generation);
		if (!actor) return;

		const alarmEvent: protocol.EventActorSetAlarm = {
			alarmTs: alarmTs !== null ? BigInt(alarmTs) : null,
		};

		const eventWrapper: protocol.EventWrapper = {
			checkpoint: {
				actorId,
				generation: actor.generation,
				index: actor.nextEventIdx++,
			},
			inner: {
				tag: "EventActorSetAlarm",
				val: alarmEvent,
			},
		};

		this.#recordEvent(eventWrapper);

		this.__sendToServer({
			tag: "ToServerEvents",
			val: [eventWrapper],
		});
	}

	clearAlarm(actorId: string, generation?: number) {
		this.setAlarm(actorId, null, generation);
	}

	#sendKvRequest(
		actorId: string,
		requestData: protocol.KvRequestData,
	): Promise<any> {
		return new Promise((resolve, reject) => {
			const requestId = this.#nextKvRequestId++;

			// Store the request
			const requestEntry = {
				actorId,
				data: requestData,
				resolve,
				reject,
				sent: false,
				timestamp: Date.now(),
			};

			this.#kvRequests.set(requestId, requestEntry);

			if (this.getPegboardWebSocketIfReady()) {
				// Send immediately
				this.#sendSingleKvRequest(requestId);
			}
		});
	}

	#sendSingleKvRequest(requestId: number) {
		const request = this.#kvRequests.get(requestId);
		if (!request || request.sent) return;

		try {
			const kvRequest: protocol.ToServerKvRequest = {
				actorId: request.actorId,
				requestId,
				data: request.data,
			};

			this.__sendToServer({
				tag: "ToServerKvRequest",
				val: kvRequest,
			});

			// Mark as sent and update timestamp
			request.sent = true;
			request.timestamp = Date.now();
		} catch (error) {
			this.#kvRequests.delete(requestId);
			request.reject(error);
		}
	}

	#processUnsentKvRequests() {
		if (!this.getPegboardWebSocketIfReady()) {
			return;
		}

		let processedCount = 0;
		for (const [requestId, request] of this.#kvRequests.entries()) {
			if (!request.sent) {
				this.#sendSingleKvRequest(requestId);
				processedCount++;
			}
		}

		if (processedCount > 0) {
			//this.#log?.log(`Processed ${processedCount} queued KV requests`);
		}
	}

	/** Asserts WebSocket exists and is ready. */
	getPegboardWebSocketIfReady(): WebSocket | undefined {
		if (
			!!this.#pegboardWebSocket &&
			this.#pegboardWebSocket.readyState === 1
		) {
			return this.#pegboardWebSocket;
		} else {
			return undefined;
		}
	}

	__sendToServer(message: protocol.ToServer) {
		this.log?.debug({
			msg: "sending runner message",
			data: stringifyToServer(message),
		});

		const encoded = protocol.encodeToServer(message);
		const pegboardWebSocket = this.getPegboardWebSocketIfReady();
		if (pegboardWebSocket) {
			pegboardWebSocket.send(encoded);
		} else {
			this.log?.error({
				msg: "WebSocket not available or not open for sending data",
			});
		}
	}

	sendHibernatableWebSocketMessageAck(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		index: number,
	) {
		if (!this.#tunnel)
			throw new Error("missing tunnel to send message ack");
		this.#tunnel.sendHibernatableWebSocketMessageAck(
			gatewayId,
			requestId,
			index,
		);
	}

	/**
	 * Restores hibernatable WebSocket connections for an actor.
	 *
	 * This method should be called at the end of `onActorStart` after the
	 * actor instance is fully initialized.
	 *
	 * This method will:
	 * - Restore all provided hibernatable WebSocket connections
	 * - Attach event listeners to the restored WebSockets
	 * - Close any WebSocket connections that failed to restore
	 *
	 * The provided metadata list should include all hibernatable WebSockets
	 * that were persisted for this actor. The gateway will automatically
	 * close any connections that are not restored (i.e., not included in
	 * this list).
	 *
	 * **Important:** This method must be called after `onActorStart` completes
	 * and before marking the actor as "ready" to ensure all hibernatable
	 * connections are fully restored.
	 *
	 * @param actorId - The ID of the actor to restore connections for
	 * @param metaEntries - Array of hibernatable WebSocket metadata to restore
	 */
	async restoreHibernatingRequests(
		actorId: string,
		metaEntries: HibernatingWebSocketMetadata[],
	) {
		if (!this.#tunnel)
			throw new Error("missing tunnel to restore hibernating requests");
		await this.#tunnel.restoreHibernatingRequests(actorId, metaEntries);
	}

	getServerlessInitPacket(): string | undefined {
		if (!this.runnerId) return undefined;

		const data = protocol.encodeToServerlessServer({
			tag: "ToServerlessServerInit",
			val: {
				runnerId: this.runnerId,
				runnerProtocolVersion: PROTOCOL_VERSION,
			},
		});

		// Embed version
		const buffer = Buffer.alloc(data.length + 2);
		buffer.writeUInt16LE(PROTOCOL_VERSION, 0);
		Buffer.from(data).copy(buffer, 2);

		return buffer.toString("base64");
	}

	#scheduleReconnect() {
		if (this.#shutdown) {
			this.log?.debug({
				msg: "Runner is shut down, not attempting reconnect",
			});
			return;
		}

		const delay = calculateBackoff(this.#reconnectAttempt, {
			initialDelay: 1000,
			maxDelay: 30000,
			multiplier: 2,
			jitter: true,
		});

		this.log?.debug({
			msg: `Scheduling reconnect attempt ${this.#reconnectAttempt + 1} in ${delay}ms`,
		});

		if (this.#reconnectTimeout) {
			this.log?.info(
				"clearing previous reconnect timeout in schedule reconnect",
			);
			clearTimeout(this.#reconnectTimeout);
		}

		this.#reconnectTimeout = setTimeout(() => {
			if (!this.#shutdown) {
				this.#reconnectAttempt++;
				this.log?.debug({
					msg: `Attempting to reconnect (attempt ${this.#reconnectAttempt})...`,
				});
				this.#openPegboardWebSocket().catch((err) => {
					this.log?.error({
						msg: "error during websocket reconnection",
						error: stringifyError(err),
					});
				});
			}
		}, delay);
	}

	#resendUnacknowledgedEvents() {
		const eventsToResend = [];

		for (const [_, actor] of this.#actors) {
			eventsToResend.push(...actor.eventHistory);
		}

		if (eventsToResend.length === 0) return;

		this.log?.info({
			msg: "resending unacknowledged events",
			count: eventsToResend.length,
		});

		// Resend events in batches
		this.__sendToServer({
			tag: "ToServerEvents",
			val: eventsToResend,
		});
	}

	#cleanupOldKvRequests() {
		const thirtySecondsAgo = Date.now() - KV_EXPIRE;
		const toDelete: number[] = [];

		for (const [requestId, request] of this.#kvRequests.entries()) {
			if (request.timestamp < thirtySecondsAgo) {
				request.reject(
					new Error(
						"KV request timed out waiting for WebSocket connection",
					),
				);
				toDelete.push(requestId);
			}
		}

		for (const requestId of toDelete) {
			this.#kvRequests.delete(requestId);
		}

		if (toDelete.length > 0) {
			//this.#log?.log(`Cleaned up ${toDelete.length} expired KV requests`);
		}
	}

	getProtocolMetadata(): protocol.ProtocolMetadata | undefined {
		return this.#protocolMetadata;
	}
}
