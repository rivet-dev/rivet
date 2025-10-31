import * as protocol from "@rivetkit/engine-runner-protocol";
import type { Logger } from "pino";
import type WebSocket from "ws";
import { logger, setLogger } from "./log.js";
import { Tunnel } from "./tunnel";
import { calculateBackoff, unreachable } from "./utils";
import { importWebSocket } from "./websocket.js";
import type { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter";

const KV_EXPIRE: number = 30_000;
const PROTOCOL_VERSION: number = 1;

/** Warn once the backlog significantly exceeds the server's ack batch size. */
const EVENT_BACKLOG_WARN_THRESHOLD = 10_000;
const SIGNAL_HANDLERS: (() => void)[] = [];

export interface ActorInstance {
	actorId: string;
	generation: number;
	config: ActorConfig;
	requests: Set<string>; // Track active request IDs
	webSockets: Set<string>; // Track active WebSocket IDs
}

export interface ActorConfig {
	name: string;
	key: string | null;
	createTs: bigint;
	input: Uint8Array | null;
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
	onDisconnected: () => void;
	onShutdown: () => void;
	fetch: (
		runner: Runner,
		actorId: string,
		request: Request,
	) => Promise<Response>;
	websocket?: (
		runner: Runner,
		actorId: string,
		ws: any,
		request: Request,
	) => Promise<void>;
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

	#actors: Map<string, ActorInstance> = new Map();
	#actorWebSockets: Map<string, Set<WebSocketTunnelAdapter>> = new Map();

	// WebSocket
	#pegboardWebSocket?: WebSocket;
	runnerId?: string;
	#lastCommandIdx: number = -1;
	#pingLoop?: NodeJS.Timeout;
	#nextEventIdx: bigint = 0n;
	#started: boolean = false;
	#shutdown: boolean = false;
	#reconnectAttempt: number = 0;
	#reconnectTimeout?: NodeJS.Timeout;

	// Runner lost threshold management
	#runnerLostThreshold?: number;
	#runnerLostTimeout?: NodeJS.Timeout;

	// Event storage for resending
	#eventHistory: protocol.EventWrapper[] = [];
	#eventBacklogWarned: boolean = false;

	// Command acknowledgment
	#ackInterval?: NodeJS.Timeout;

	// KV operations
	#nextRequestId: number = 0;
	#kvRequests: Map<number, KvRequestEntry> = new Map();
	#kvCleanupInterval?: NodeJS.Timeout;

	// Tunnel for HTTP/WebSocket forwarding
	#tunnel: Tunnel | undefined;

	constructor(config: RunnerConfig) {
		this.#config = config;
		if (this.#config.logger) setLogger(this.#config.logger);

		// Start cleaning up old unsent KV requests every 15 seconds
		this.#kvCleanupInterval = setInterval(() => {
			this.#cleanupOldKvRequests();
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
		const actor = this.#removeActor(actorId, generation);
		if (!actor) return;

		// If onActorStop times out, Pegboard will handle this timeout with ACTOR_STOP_THRESHOLD_DURATION_MS
		try {
			await this.#config.onActorStop(actorId, actor.generation);
		} catch (err) {
			console.error(`Error in onActorStop for actor ${actorId}:`, err);
		}

		this.#sendActorStateUpdate(actorId, actor.generation, "stopped");

		this.#config.onActorStop(actorId, actor.generation).catch((err) => {
			logger()?.error({
				msg: "error in onactorstop for actor",
				runnerId: this.runnerId,
				actorId,
				err,
			});
		});
	}

	#stopAllActors() {
		logger()?.info({
			msg: "stopping all actors due to runner lost threshold exceeded",
			runnerId: this.runnerId,
		});

		const actorIds = Array.from(this.#actors.keys());
		for (const actorId of actorIds) {
			this.forceStopActor(actorId);
		}
	}

	getActor(actorId: string, generation?: number): ActorInstance | undefined {
		const actor = this.#actors.get(actorId);
		if (!actor) {
			logger()?.error({
				msg: "actor not found",
				runnerId: this.runnerId,
				actorId,
			});
			return undefined;
		}
		if (generation !== undefined && actor.generation !== generation) {
			logger()?.error({
				msg: "actor generation mismatch",
				runnerId: this.runnerId,
				actorId,
				generation,
			});
			return undefined;
		}

		return actor;
	}

	hasActor(actorId: string, generation?: number): boolean {
		const actor = this.#actors.get(actorId);

		return (
			!!actor &&
			(generation === undefined || actor.generation === generation)
		);
	}

	#removeActor(
		actorId: string,
		generation?: number,
	): ActorInstance | undefined {
		const actor = this.#actors.get(actorId);
		if (!actor) {
			logger()?.error({
				msg: "actor not found for removal",
				runnerId: this.runnerId,
				actorId,
			});
			return undefined;
		}
		if (generation !== undefined && actor.generation !== generation) {
			logger()?.error({
				msg: "actor generation mismatch",
				runnerId: this.runnerId,
				actorId,
				generation,
			});
			return undefined;
		}

		this.#actors.delete(actorId);

		// Unregister actor from tunnel
		this.#tunnel?.unregisterActor(actor);

		return actor;
	}

	// MARK: Start
	async start() {
		if (this.#started) throw new Error("Cannot call runner.start twice");
		this.#started = true;

		logger()?.info({ msg: "starting runner" });

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
				process.on("SIGTERM", () => {
					logger()?.debug("received SIGTERM");

					for (const handler of SIGNAL_HANDLERS) {
						handler();
					}

					process.exit(0);
				});
				process.on("SIGINT", () => {
					logger()?.debug("received SIGINT");

					for (const handler of SIGNAL_HANDLERS) {
						handler();
					}

					process.exit(0);
				});

				logger()?.debug({
					msg: "added SIGTERM listeners",
				});
			}

			SIGNAL_HANDLERS.push(() => {
				const weak = new WeakRef(this);
				weak.deref()?.shutdown(false, false);
			});
		}
	}

	// MARK: Shutdown
	async shutdown(immediate: boolean, exit: boolean = false) {
		logger()?.info({
			msg: "starting shutdown",
			runnerId: this.runnerId,
			immediate,
			exit,
		});
		this.#shutdown = true;

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

		// Clear ping loop
		if (this.#pingLoop) {
			clearInterval(this.#pingLoop);
			this.#pingLoop = undefined;
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
		if (
			this.#pegboardWebSocket &&
			this.#pegboardWebSocket.readyState === 1
		) {
			const pegboardWebSocket = this.#pegboardWebSocket;
			if (immediate) {
				// Stop immediately
				pegboardWebSocket.close(1000, "Stopping");
			} else {
				// Wait for actors to shut down before stopping
				try {
					logger()?.info({
						msg: "sending stopping message",
						runnerId: this.runnerId,
						readyState: pegboardWebSocket.readyState,
					});

					// NOTE: We don't use #sendToServer here because that function checks if the runner is
					// shut down
					const encoded = protocol.encodeToServer({
						tag: "ToServerStopping",
						val: null,
					});
					if (
						this.#pegboardWebSocket &&
						this.#pegboardWebSocket.readyState === 1
					) {
						this.#pegboardWebSocket.send(encoded);
					} else {
						logger()?.error(
							"WebSocket not available or not open for sending data",
						);
					}

					const closePromise = new Promise<void>((resolve) => {
						if (!pegboardWebSocket)
							throw new Error("missing pegboardWebSocket");

						pegboardWebSocket.addEventListener("close", (ev) => {
							logger()?.info({
								msg: "connection closed",
								runnerId: this.runnerId,
								code: ev.code,
								reason: ev.reason.toString(),
							});
							resolve();
						});
					});

					// TODO: Wait for all actors to stop before closing ws

					logger()?.info({
						msg: "closing WebSocket",
						runnerId: this.runnerId,
					});
					pegboardWebSocket.close(1000, "Stopping");

					await closePromise;

					logger()?.info({
						msg: "websocket shutdown completed",
						runnerId: this.runnerId,
					});
				} catch (error) {
					logger()?.error({
						msg: "error during websocket shutdown:",
						runnerId: this.runnerId,
						error,
					});
					pegboardWebSocket.close();
				}
			}
		} else {
			logger()?.warn({
				msg: "no runner WebSocket to shutdown or already closed",
				runnerId: this.runnerId,
				readyState: this.#pegboardWebSocket?.readyState,
			});
		}

		// Close tunnel
		if (this.#tunnel) {
			this.#tunnel.shutdown();
			this.#tunnel = undefined;
		}

		if (exit) process.exit(0);

		this.#config.onShutdown();
	}

	// MARK: Networking
	get pegboardUrl() {
		const endpoint = this.#config.pegboardEndpoint || this.#config.endpoint;
		const wsEndpoint = endpoint
			.replace("http://", "ws://")
			.replace("https://", "wss://");
		return `${wsEndpoint}?protocol_version=${PROTOCOL_VERSION}&namespace=${encodeURIComponent(this.#config.namespace)}&runner_key=${encodeURIComponent(this.#config.runnerKey)}`;
	}

	// MARK: Runner protocol
	async #openPegboardWebSocket() {
		const protocols = ["rivet", `rivet_target.runner`];
		if (this.config.token)
			protocols.push(`rivet_token.${this.config.token}`);

		const WS = await importWebSocket();
		const ws = new WS(this.pegboardUrl, protocols) as any as WebSocket;
		this.#pegboardWebSocket = ws;

		ws.addEventListener("open", () => {
			logger()?.info({ msg: "Connected" });

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
				lastCommandIdx:
					this.#lastCommandIdx >= 0
						? BigInt(this.#lastCommandIdx)
						: null,
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

			// Process unsent KV requests
			this.#processUnsentKvRequests();

			// Start ping interval
			const pingInterval = 1000;
			const pingLoop = setInterval(() => {
				if (ws.readyState === 1) {
					this.__sendToServer({
						tag: "ToServerPing",
						val: {
							ts: BigInt(Date.now()),
						},
					});
				} else {
					clearInterval(pingLoop);
					logger()?.info({
						msg: "WebSocket not open, stopping ping loop",
						runnerId: this.runnerId,
					});
				}
			}, pingInterval);
			this.#pingLoop = pingLoop;

			// Start command acknowledgment interval (5 minutes)
			const ackInterval = 5 * 60 * 1000; // 5 minutes in milliseconds
			const ackLoop = setInterval(() => {
				if (ws.readyState === 1) {
					this.#sendCommandAcknowledgment();
				} else {
					clearInterval(ackLoop);
					logger()?.info({
						msg: "WebSocket not open, stopping ack loop",
						runnerId: this.runnerId,
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

			// Handle message
			if (message.tag === "ToClientInit") {
				const init = message.val;

				if (this.runnerId !== init.runnerId) {
					this.runnerId = init.runnerId;

					// Clear history if runner id changed
					this.#eventHistory.length = 0;
				}

				// Store the runner lost threshold from metadata
				this.#runnerLostThreshold = init.metadata?.runnerLostThreshold
					? Number(init.metadata.runnerLostThreshold)
					: undefined;

				logger()?.info({
					msg: "received init",
					runnerId: init.runnerId,
					lastEventIdx: init.lastEventIdx,
					runnerLostThreshold: this.#runnerLostThreshold,
				});

				// Resend events that haven't been acknowledged
				this.#resendUnacknowledgedEvents(init.lastEventIdx);

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
				this.#tunnel?.handleTunnelMessage(message.val);
			} else if (message.tag === "ToClientClose") {
				this.#tunnel?.shutdown();
				ws.close(1000, "manual closure");
			} else {
				unreachable(message);
			}
		});

		ws.addEventListener("error", (ev) => {
			logger()?.error({
				msg: `WebSocket error: ${ev.error}`,
				runnerId: this.runnerId,
			});

			if (!this.#shutdown) {
				// Start runner lost timeout if we have a threshold and are not shutting down
				if (
					!this.#runnerLostTimeout &&
					this.#runnerLostThreshold &&
					this.#runnerLostThreshold > 0
				) {
					logger()?.info({
						msg: "starting runner lost timeout",
						runnerId: this.runnerId,
						seconds: this.#runnerLostThreshold / 1000,
					});
					this.#runnerLostTimeout = setTimeout(() => {
						this.#stopAllActors();
					}, this.#runnerLostThreshold);
				}

				// Attempt to reconnect if not stopped
				this.#scheduleReconnect();
			}
		});

		ws.addEventListener("close", async (ev) => {
			logger()?.info({
				msg: "connection closed",
				runnerId: this.runnerId,
				code: ev.code,
				reason: ev.reason.toString(),
			});

			this.#config.onDisconnected();

			if (ev.reason.toString().startsWith("ws.eviction")) {
				logger()?.info({
					msg: "runner evicted",
					runnerId: this.runnerId,
				});

				await this.shutdown(true);
			}

			// Clear ping loop on close
			if (this.#pingLoop) {
				clearInterval(this.#pingLoop);
				this.#pingLoop = undefined;
			}

			// Clear ack interval on close
			if (this.#ackInterval) {
				clearInterval(this.#ackInterval);
				this.#ackInterval = undefined;
			}

			if (!this.#shutdown) {
				// Start runner lost timeout if we have a threshold and are not shutting down
				if (
					!this.#runnerLostTimeout &&
					this.#runnerLostThreshold &&
					this.#runnerLostThreshold > 0
				) {
					logger()?.info({
						msg: "starting runner lost timeout",
						runnerId: this.runnerId,
						seconds: this.#runnerLostThreshold / 1000,
					});
					this.#runnerLostTimeout = setTimeout(() => {
						this.#stopAllActors();
					}, this.#runnerLostThreshold);
				}

				// Attempt to reconnect if not stopped
				this.#scheduleReconnect();
			}
		});
	}

	#handleCommands(commands: protocol.ToClientCommands) {
		logger()?.info({
			msg: "received commands",
			runnerId: this.runnerId,
			commandCount: commands.length,
		});

		for (const commandWrapper of commands) {
			logger()?.info({
				msg: "received command",
				runnerId: this.runnerId,
				commandWrapper,
			});
			if (commandWrapper.inner.tag === "CommandStartActor") {
				this.#handleCommandStartActor(commandWrapper);
			} else if (commandWrapper.inner.tag === "CommandStopActor") {
				this.#handleCommandStopActor(commandWrapper);
			} else {
				unreachable(commandWrapper.inner);
			}

			this.#lastCommandIdx = Number(commandWrapper.index);
		}
	}

	#handleAckEvents(ack: protocol.ToClientAckEvents) {
		const lastAckedIdx = ack.lastEventIdx;

		const originalLength = this.#eventHistory.length;
		this.#eventHistory = this.#eventHistory.filter(
			(event) => event.index > lastAckedIdx,
		);

		const prunedCount = originalLength - this.#eventHistory.length;
		if (prunedCount > 0) {
			logger()?.info({
				msg: "pruned acknowledged events",
				runnerId: this.runnerId,
				lastAckedIdx: lastAckedIdx.toString(),
				prunedCount,
			});
		}

		if (this.#eventHistory.length <= EVENT_BACKLOG_WARN_THRESHOLD) {
			this.#eventBacklogWarned = false;
		}
	}

	/** Track events to send to the server in case we need to resend it on disconnect. */
	#recordEvent(eventWrapper: protocol.EventWrapper) {
		this.#eventHistory.push(eventWrapper);

		if (
			this.#eventHistory.length > EVENT_BACKLOG_WARN_THRESHOLD &&
			!this.#eventBacklogWarned
		) {
			this.#eventBacklogWarned = true;
			logger()?.warn({
				msg: "unacknowledged event backlog exceeds threshold",
				runnerId: this.runnerId,
				backlogSize: this.#eventHistory.length,
				threshold: EVENT_BACKLOG_WARN_THRESHOLD,
			});
		}
	}

	#handleCommandStartActor(commandWrapper: protocol.CommandWrapper) {
		const startCommand = commandWrapper.inner
			.val as protocol.CommandStartActor;

		const actorId = startCommand.actorId;
		const generation = startCommand.generation;
		const config = startCommand.config;

		const actorConfig: ActorConfig = {
			name: config.name,
			key: config.key,
			createTs: config.createTs,
			input: config.input ? new Uint8Array(config.input) : null,
		};

		const instance: ActorInstance = {
			actorId,
			generation,
			config: actorConfig,
			requests: new Set(),
			webSockets: new Set(),
		};

		this.#actors.set(actorId, instance);

		this.#sendActorStateUpdate(actorId, generation, "running");

		// TODO: Add timeout to onActorStart
		// Call onActorStart asynchronously and handle errors
		this.#config
			.onActorStart(actorId, generation, actorConfig)
			.catch((err) => {
				logger()?.error({
					msg: "error in onactorstart for actor",
					runnerId: this.runnerId,
					actorId,
					err,
				});

				// TODO: Mark as crashed
				// Send stopped state update if start failed
				this.forceStopActor(actorId, generation);
			});
	}

	#handleCommandStopActor(commandWrapper: protocol.CommandWrapper) {
		const stopCommand = commandWrapper.inner
			.val as protocol.CommandStopActor;

		const actorId = stopCommand.actorId;
		const generation = stopCommand.generation;

		this.forceStopActor(actorId, generation);
	}

	#sendActorIntent(
		actorId: string,
		generation: number,
		intentType: "sleep" | "stop",
	) {
		if (this.#shutdown) {
			logger()?.warn({
				msg: "Runner is shut down, cannot send actor intent",
				runnerId: this.runnerId,
			});
			return;
		}
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
			actorId,
			generation,
			intent: actorIntent,
		};

		const eventIndex = this.#nextEventIdx++;
		const eventWrapper: protocol.EventWrapper = {
			index: eventIndex,
			inner: {
				tag: "EventActorIntent",
				val: intentEvent,
			},
		};

		this.#recordEvent(eventWrapper);

		logger()?.info({
			msg: "sending event to server",
			runnerId: this.runnerId,
			index: eventWrapper.index,
			tag: eventWrapper.inner.tag,
			val: eventWrapper.inner.val,
		});

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
		if (this.#shutdown) {
			logger()?.warn({
				msg: "Runner is shut down, cannot send actor state update",
				runnerId: this.runnerId,
			});
			return;
		}
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
			actorId,
			generation,
			state: actorState,
		};

		const eventIndex = this.#nextEventIdx++;
		const eventWrapper: protocol.EventWrapper = {
			index: eventIndex,
			inner: {
				tag: "EventActorStateUpdate",
				val: stateUpdateEvent,
			},
		};

		this.#recordEvent(eventWrapper);

		logger()?.info({
			msg: "sending event to server",
			runnerId: this.runnerId,
			index: eventWrapper.index,
			tag: eventWrapper.inner.tag,
			val: eventWrapper.inner.val,
		});

		this.__sendToServer({
			tag: "ToServerEvents",
			val: [eventWrapper],
		});
	}

	#sendCommandAcknowledgment() {
		if (this.#shutdown) {
			logger()?.warn({
				msg: "Runner is shut down, cannot send command acknowledgment",
				runnerId: this.runnerId,
			});
			return;
		}

		if (this.#lastCommandIdx < 0) {
			// No commands received yet, nothing to acknowledge
			return;
		}

		//logger()?.log("Sending command acknowledgment", this.#lastCommandIdx);

		this.__sendToServer({
			tag: "ToServerAckCommands",
			val: {
				lastCommandIdx: BigInt(this.#lastCommandIdx),
			},
		});
	}

	#handleKvResponse(response: protocol.ToClientKvResponse) {
		const requestId = response.requestId;
		const request = this.#kvRequests.get(requestId);

		if (!request) {
			const msg = "received kv response for unknown request id";
			logger()?.error({ msg, runnerId: this.runnerId, requestId });
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

		if (this.#shutdown) {
			console.warn("Runner is shut down, cannot set alarm");
			return;
		}

		const alarmEvent: protocol.EventActorSetAlarm = {
			actorId,
			generation: actor.generation,
			alarmTs: alarmTs !== null ? BigInt(alarmTs) : null,
		};

		const eventIndex = this.#nextEventIdx++;
		const eventWrapper: protocol.EventWrapper = {
			index: eventIndex,
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
			if (this.#shutdown) {
				reject(new Error("Runner is shut down"));
				return;
			}

			const requestId = this.#nextRequestId++;
			const isConnected =
				this.#pegboardWebSocket &&
				this.#pegboardWebSocket.readyState === 1;

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

			if (isConnected) {
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
		if (
			!this.#pegboardWebSocket ||
			this.#pegboardWebSocket.readyState !== 1
		) {
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
			//logger()?.log(`Processed ${processedCount} queued KV requests`);
		}
	}

	__webSocketReady(): boolean {
		return this.#pegboardWebSocket
			? this.#pegboardWebSocket.readyState === 1
			: false;
	}

	__sendToServer(message: protocol.ToServer) {
		if (this.#shutdown) {
			logger()?.warn({
				msg: "Runner is shut down, cannot send message to server",
				runnerId: this.runnerId,
			});
			return;
		}

		logger()?.debug({
			msg: "------------ SEND",
		});

		const encoded = protocol.encodeToServer(message);
		if (
			this.#pegboardWebSocket &&
			this.#pegboardWebSocket.readyState === 1
		) {
			logger()?.debug({
				msg: "------------ SEND 2",
			});
			this.#pegboardWebSocket.send(encoded);
			logger()?.debug({
				msg: "------------ SEND 3",
			});
		} else {
			logger()?.error({
				msg: "WebSocket not available or not open for sending data",
				runnerId: this.runnerId,
			});
		}
	}

	getServerlessInitPacket(): string | undefined {
		if (!this.runnerId) return undefined;

		const data = protocol.encodeToServerlessServer({
			tag: "ToServerlessServerInit",
			val: {
				runnerId: this.runnerId,
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
			logger()?.debug({
				msg: "Runner is shut down, not attempting reconnect",
				runnerId: this.runnerId,
			});
			return;
		}

		const delay = calculateBackoff(this.#reconnectAttempt, {
			initialDelay: 1000,
			maxDelay: 30000,
			multiplier: 2,
			jitter: true,
		});

		logger()?.debug({
			msg: `Scheduling reconnect attempt ${this.#reconnectAttempt + 1} in ${delay}ms`,
			runnerId: this.runnerId,
		});

		this.#reconnectTimeout = setTimeout(async () => {
			if (!this.#shutdown) {
				this.#reconnectAttempt++;
				logger()?.debug({
					msg: `Attempting to reconnect (attempt ${this.#reconnectAttempt})...`,
					runnerId: this.runnerId,
				});
				await this.#openPegboardWebSocket();
			}
		}, delay);
	}

	#resendUnacknowledgedEvents(lastEventIdx: bigint) {
		const eventsToResend = this.#eventHistory.filter(
			(event) => event.index > lastEventIdx,
		);

		if (eventsToResend.length === 0) return;

		//logger()?.log(
		//	`Resending ${eventsToResend.length} unacknowledged events from index ${Number(lastEventIdx) + 1}`,
		//);

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
			//logger()?.log(`Cleaned up ${toDelete.length} expired KV requests`);
		}
	}
}
