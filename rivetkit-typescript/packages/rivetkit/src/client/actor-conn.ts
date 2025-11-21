import * as cbor from "cbor-x";
import invariant from "invariant";
import pRetry from "p-retry";
import type { CloseEvent } from "ws";
import type { AnyActorDefinition } from "@/actor/definition";
import { inputDataToBuffer } from "@/actor/protocol/old";
import { type Encoding, jsonStringifyCompat } from "@/actor/protocol/serde";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
	PATH_CONNECT,
} from "@/common/actor-router-consts";
import { importEventSource } from "@/common/eventsource";
import type {
	UniversalErrorEvent,
	UniversalEventSource,
	UniversalMessageEvent,
} from "@/common/eventsource-interface";
import { assertUnreachable, stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { ManagerDriver } from "@/driver-helpers/mod";
import type { ActorQuery } from "@/manager/protocol/query";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	TO_CLIENT_VERSIONED,
	TO_SERVER_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type ToClient as ToClientJson,
	ToClientSchema,
	type ToServer as ToServerJson,
	ToServerSchema,
} from "@/schemas/client-protocol-zod/mod";
import {
	deserializeWithEncoding,
	encodingIsBinary,
	serializeWithEncoding,
} from "@/serde";
import {
	bufferToArrayBuffer,
	getEnvUniversal,
	httpUserAgent,
	promiseWithResolvers,
} from "@/utils";
import type { ActorDefinitionActions } from "./actor-common";
import { queryActor } from "./actor-query";
import { ACTOR_CONNS_SYMBOL, type ClientRaw } from "./client";
import * as errors from "./errors";
import { logger } from "./log";
import {
	type WebSocketMessage as ConnMessage,
	messageLength,
	sendHttpRequest,
} from "./utils";

interface ActionInFlight {
	name: string;
	resolve: (response: { id: bigint; output: unknown }) => void;
	reject: (error: Error) => void;
}

interface EventSubscriptions<Args extends Array<unknown>> {
	callback: (...args: Args) => void;
	once: boolean;
}

/**
 * A function that unsubscribes from an event.
 *
 * @typedef {Function} EventUnsubscribe
 */
export type EventUnsubscribe = () => void;

/**
 * A function that handles connection errors.
 *
 * @typedef {Function} ActorErrorCallback
 */
export type ActorErrorCallback = (error: errors.ActorError) => void;

export interface SendHttpMessageOpts {
	ephemeral: boolean;
	signal?: AbortSignal;
}

export const CONNECT_SYMBOL = Symbol("connect");

/**
 * Provides underlying functions for {@link ActorConn}. See {@link ActorConn} for using type-safe remote procedure calls.
 *
 * @see {@link ActorConn}
 */
export class ActorConnRaw {
	#disposed = false;

	/* Will be aborted on dispose. */
	#abortController = new AbortController();

	#connecting = false;

	#actorId?: string;
	#connectionId?: string;

	#messageQueue: Array<{
		body:
			| {
					tag: "ActionRequest";
					val: { id: bigint; name: string; args: unknown };
			  }
			| {
					tag: "SubscriptionRequest";
					val: { eventName: string; subscribe: boolean };
			  };
	}> = [];
	#actionsInFlight = new Map<number, ActionInFlight>();

	// biome-ignore lint/suspicious/noExplicitAny: Unknown subscription type
	#eventSubscriptions = new Map<string, Set<EventSubscriptions<any[]>>>();

	#errorHandlers = new Set<ActorErrorCallback>();

	#actionIdCounter = 0;

	/**
	 * Interval that keeps the NodeJS process alive if this is the only thing running.
	 *
	 * See ttps://github.com/nodejs/node/issues/22088
	 */
	#keepNodeAliveInterval: NodeJS.Timeout;

	/** Promise used to indicate the socket has connected successfully. This will be rejected if the connection fails. */
	#onOpenPromise?: ReturnType<typeof promiseWithResolvers<undefined>>;

	#websocket?: UniversalWebSocket;

	#client: ClientRaw;
	#driver: ManagerDriver;
	#params: unknown;
	#encoding: Encoding;
	#actorQuery: ActorQuery;

	// TODO: ws message queue

	/**
	 * Do not call this directly.
	 *
	 * Creates an instance of ActorConnRaw.
	 *
	 * @protected
	 */
	public constructor(
		client: ClientRaw,
		driver: ManagerDriver,
		params: unknown,
		encoding: Encoding,
		actorQuery: ActorQuery,
	) {
		this.#client = client;
		this.#driver = driver;
		this.#params = params;
		this.#encoding = encoding;
		this.#actorQuery = actorQuery;

		this.#keepNodeAliveInterval = setInterval(() => 60_000);
	}

	/**
	 * Call a raw action connection. See {@link ActorConn} for type-safe action calls.
	 *
	 * @see {@link ActorConn}
	 * @template Args - The type of arguments to pass to the action function.
	 * @template Response - The type of the response returned by the action function.
	 * @param {string} name - The name of the action function to call.
	 * @param {...Args} args - The arguments to pass to the action function.
	 * @returns {Promise<Response>} - A promise that resolves to the response of the action function.
	 */
	async action<
		Args extends Array<unknown> = unknown[],
		Response = unknown,
	>(opts: {
		name: string;
		args: Args;
		signal?: AbortSignal;
	}): Promise<Response> {
		logger().debug({ msg: "action", name: opts.name, args: opts.args });

		// If we have an active connection, use the websockactionId
		const actionId = this.#actionIdCounter;
		this.#actionIdCounter += 1;

		const { promise, resolve, reject } = promiseWithResolvers<{
			id: bigint;
			output: unknown;
		}>();
		this.#actionsInFlight.set(actionId, {
			name: opts.name,
			resolve,
			reject,
		});
		logger().debug({
			msg: "added action to in-flight map",
			actionId,
			actionName: opts.name,
			inFlightCount: this.#actionsInFlight.size,
		});

		this.#sendMessage({
			body: {
				tag: "ActionRequest",
				val: {
					id: BigInt(actionId),
					name: opts.name,
					args: opts.args,
				},
			},
		});

		// TODO: Throw error if disconnect is called

		const { id: responseId, output } = await promise;
		if (responseId !== BigInt(actionId))
			throw new Error(
				`Request ID ${actionId} does not match response ID ${responseId}`,
			);

		return output as Response;
	}

	/**
	 * Do not call this directly.
enc
	 * Establishes a connection to the server using the specified endpoint & encoding & driver.
	 *
	 * @protected
	 */
	public [CONNECT_SYMBOL]() {
		this.#connectWithRetry();
	}

	async #connectWithRetry() {
		this.#connecting = true;

		// Attempt to reconnect indefinitely
		try {
			await pRetry(this.#connectAndWait.bind(this), {
				forever: true,
				minTimeout: 250,
				maxTimeout: 30_000,

				onFailedAttempt: (error) => {
					logger().warn({
						msg: "failed to reconnect",
						attempt: error.attemptNumber,
						error: stringifyError(error),
					});
				},

				// Cancel retry if aborted
				signal: this.#abortController.signal,
			});
		} catch (err) {
			if ((err as Error).name === "AbortError") {
				// Ignore abortions
				logger().info({ msg: "connection retry aborted" });
				return;
			} else {
				// Unknown error
				throw err;
			}
		}

		this.#connecting = false;
	}

	async #connectAndWait() {
		try {
			// Create promise for open
			if (this.#onOpenPromise)
				throw new Error("#onOpenPromise already defined");
			this.#onOpenPromise = promiseWithResolvers();

			await this.#connectWebSocket();

			// Wait for result
			await this.#onOpenPromise.promise;
		} finally {
			this.#onOpenPromise = undefined;
		}
	}

	async #connectWebSocket() {
		const { actorId } = await queryActor(
			undefined,
			this.#actorQuery,
			this.#driver,
		);

		const ws = await this.#driver.openWebSocket(
			PATH_CONNECT,
			actorId,
			this.#encoding,
			this.#params,
		);
		logger().debug({
			msg: "opened websocket",
			connectionId: this.#connectionId,
			readyState: ws.readyState,
			messageQueueLength: this.#messageQueue.length,
		});
		this.#websocket = ws;
		ws.addEventListener("open", () => {
			logger().debug({
				msg: "client websocket open",
				connectionId: this.#connectionId,
			});
		});
		ws.addEventListener("message", async (ev) => {
			try {
				await this.#handleOnMessage(ev.data);
			} catch (err) {
				logger().error({
					msg: "error in websocket message handler",
					error: stringifyError(err),
				});
			}
		});
		ws.addEventListener("close", (ev) => {
			try {
				this.#handleOnClose(ev);
			} catch (err) {
				logger().error({
					msg: "error in websocket close handler",
					error: stringifyError(err),
				});
			}
		});
		ws.addEventListener("error", (_ev) => {
			try {
				this.#handleOnError();
			} catch (err) {
				logger().error({
					msg: "error in websocket error handler",
					error: stringifyError(err),
				});
			}
		});
	}

	/** Called by the onopen event from drivers. */
	#handleOnOpen() {
		logger().debug({
			msg: "socket open",
			messageQueueLength: this.#messageQueue.length,
			connectionId: this.#connectionId,
		});

		// Resolve open promise
		if (this.#onOpenPromise) {
			this.#onOpenPromise.resolve(undefined);
		} else {
			logger().warn({ msg: "#onOpenPromise is undefined" });
		}

		// Resubscribe to all active events
		for (const eventName of this.#eventSubscriptions.keys()) {
			this.#sendSubscription(eventName, true);
		}

		// Flush queue
		//
		// If the message fails to send, the message will be re-queued
		const queue = this.#messageQueue;
		this.#messageQueue = [];
		logger().debug({
			msg: "flushing message queue",
			queueLength: queue.length,
		});
		for (const msg of queue) {
			this.#sendMessage(msg);
		}
	}

	/** Called by the onmessage event from drivers. */
	async #handleOnMessage(data: any) {
		logger().trace({
			msg: "received message",
			dataType: typeof data,
			isBlob: data instanceof Blob,
			isArrayBuffer: data instanceof ArrayBuffer,
		});

		const response = await this.#parseMessage(data as ConnMessage);
		logger().trace(
			getEnvUniversal("_RIVETKIT_LOG_MESSAGE")
				? {
						msg: "parsed message",
						message:
							jsonStringifyCompat(response).substring(0, 100) +
							"...",
					}
				: { msg: "parsed message" },
		);

		if (response.body.tag === "Init") {
			// Store connection info
			this.#actorId = response.body.val.actorId;
			this.#connectionId = response.body.val.connectionId;
			logger().trace({
				msg: "received init message",
				actorId: this.#actorId,
				connectionId: this.#connectionId,
			});
			this.#handleOnOpen();
		} else if (response.body.tag === "Error") {
			// Connection error
			const { group, code, message, metadata, actionId } =
				response.body.val;

			if (actionId) {
				const inFlight = this.#takeActionInFlight(Number(actionId));

				logger().warn({
					msg: "action error",
					actionId: actionId,
					actionName: inFlight?.name,
					group,
					code,
					message,
					metadata,
				});

				inFlight.reject(
					new errors.ActorError(group, code, message, metadata),
				);
			} else {
				logger().warn({
					msg: "connection error",
					group,
					code,
					message,
					metadata,
				});

				// Create a connection error
				const actorError = new errors.ActorError(
					group,
					code,
					message,
					metadata,
				);

				// If we have an onOpenPromise, reject it with the error
				if (this.#onOpenPromise) {
					this.#onOpenPromise.reject(actorError);
				}

				// Reject any in-flight requests
				for (const [id, inFlight] of this.#actionsInFlight.entries()) {
					inFlight.reject(actorError);
					this.#actionsInFlight.delete(id);
				}

				// Dispatch to error handler if registered
				this.#dispatchActorError(actorError);
			}
		} else if (response.body.tag === "ActionResponse") {
			// Action response OK
			const { id: actionId } = response.body.val;
			logger().debug({
				msg: "received action response",
				actionId: Number(actionId),
				inFlightCount: this.#actionsInFlight.size,
				inFlightIds: Array.from(this.#actionsInFlight.keys()),
			});

			const inFlight = this.#takeActionInFlight(Number(actionId));
			logger().trace({
				msg: "resolving action promise",
				actionId,
				actionName: inFlight?.name,
			});
			inFlight.resolve(response.body.val);
		} else if (response.body.tag === "Event") {
			logger().trace({
				msg: "received event",
				name: response.body.val.name,
			});
			this.#dispatchEvent(response.body.val);
		} else {
			assertUnreachable(response.body);
		}
	}

	/** Called by the onclose event from drivers. */
	#handleOnClose(event: Event | CloseEvent) {
		// TODO: Handle queue
		// TODO: Reconnect with backoff

		// We can't use `event instanceof CloseEvent` because it's not defined in NodeJS
		//
		// These properties will be undefined
		const closeEvent = event as CloseEvent;
		const wasClean = closeEvent.wasClean;

		// Reject open promise
		if (this.#onOpenPromise) {
			this.#onOpenPromise.reject(
				new Error(
					`websocket closed with code ${closeEvent.code}: ${closeEvent.reason}`,
				),
			);
		}

		logger().info({
			msg: "socket closed",
			code: closeEvent.code,
			reason: closeEvent.reason,
			wasClean: wasClean,
			connectionId: this.#connectionId,
			messageQueueLength: this.#messageQueue.length,
			actionsInFlight: this.#actionsInFlight.size,
		});

		// Reject all in-flight actions
		if (this.#actionsInFlight.size > 0) {
			logger().debug({
				msg: "rejecting in-flight actions after disconnect",
				count: this.#actionsInFlight.size,
				connectionId: this.#connectionId,
				wasClean,
			});

			const disconnectError = new Error(
				wasClean ? "Connection closed" : "Connection lost",
			);

			for (const actionInfo of this.#actionsInFlight.values()) {
				actionInfo.reject(disconnectError);
			}
			this.#actionsInFlight.clear();
		}

		this.#websocket = undefined;

		// Automatically reconnect. Skip if already attempting to connect.
		if (!this.#disposed && !this.#connecting) {
			logger().debug({
				msg: "triggering reconnect",
				connectionId: this.#connectionId,
				messageQueueLength: this.#messageQueue.length,
			});
			// TODO: Fetch actor to check if it's destroyed
			// TODO: Add backoff for reconnect
			// TODO: Add a way of preserving connection ID for connection state

			// Attempt to connect again
			this.#connectWithRetry();
		}
	}

	/** Called by the onerror event from drivers. */
	#handleOnError() {
		if (this.#disposed) return;

		// More detailed information will be logged in onclose
		logger().warn("socket error");
	}

	#takeActionInFlight(id: number): ActionInFlight {
		const inFlight = this.#actionsInFlight.get(id);
		if (!inFlight) {
			logger().error({
				msg: "action not found in in-flight map",
				lookupId: id,
				inFlightCount: this.#actionsInFlight.size,
				inFlightIds: Array.from(this.#actionsInFlight.keys()),
				inFlightActions: Array.from(
					this.#actionsInFlight.entries(),
				).map(([id, action]) => ({
					id,
					name: action.name,
				})),
			});
			throw new errors.InternalError(`No in flight response for ${id}`);
		}
		this.#actionsInFlight.delete(id);
		logger().debug({
			msg: "removed action from in-flight map",
			actionId: id,
			actionName: inFlight.name,
			inFlightCount: this.#actionsInFlight.size,
		});
		return inFlight;
	}

	#dispatchEvent(event: { name: string; args: unknown }) {
		const { name, args } = event;

		const listeners = this.#eventSubscriptions.get(name);
		if (!listeners) return;

		// Create a new array to avoid issues with listeners being removed during iteration
		for (const listener of [...listeners]) {
			listener.callback(...(args as unknown[]));

			// Remove if this was a one-time listener
			if (listener.once) {
				listeners.delete(listener);
			}
		}

		// Clean up empty listener sets
		if (listeners.size === 0) {
			this.#eventSubscriptions.delete(name);
		}
	}

	#dispatchActorError(error: errors.ActorError) {
		// Call all registered error handlers
		for (const handler of [...this.#errorHandlers]) {
			try {
				handler(error);
			} catch (err) {
				logger().error({
					msg: "error in connection error handler",
					error: stringifyError(err),
				});
			}
		}
	}

	#addEventSubscription<Args extends Array<unknown>>(
		eventName: string,
		callback: (...args: Args) => void,
		once: boolean,
	): EventUnsubscribe {
		const listener: EventSubscriptions<Args> = {
			callback,
			once,
		};

		let subscriptionSet = this.#eventSubscriptions.get(eventName);
		if (subscriptionSet === undefined) {
			subscriptionSet = new Set();
			this.#eventSubscriptions.set(eventName, subscriptionSet);
			this.#sendSubscription(eventName, true);
		}
		subscriptionSet.add(listener);

		// Return unsubscribe function
		return () => {
			const listeners = this.#eventSubscriptions.get(eventName);
			if (listeners) {
				listeners.delete(listener);
				if (listeners.size === 0) {
					this.#eventSubscriptions.delete(eventName);
					this.#sendSubscription(eventName, false);
				}
			}
		};
	}

	/**
	 * Subscribes to an event that will happen repeatedly.
	 *
	 * @template Args - The type of arguments the event callback will receive.
	 * @param {string} eventName - The name of the event to subscribe to.
	 * @param {(...args: Args) => void} callback - The callback function to execute when the event is triggered.
	 * @returns {EventUnsubscribe} - A function to unsubscribe from the event.
	 * @see {@link https://rivet.dev/docs/events|Events Documentation}
	 */
	on<Args extends Array<unknown> = unknown[]>(
		eventName: string,
		callback: (...args: Args) => void,
	): EventUnsubscribe {
		return this.#addEventSubscription<Args>(eventName, callback, false);
	}

	/**
	 * Subscribes to an event that will be triggered only once.
	 *
	 * @template Args - The type of arguments the event callback will receive.
	 * @param {string} eventName - The name of the event to subscribe to.
	 * @param {(...args: Args) => void} callback - The callback function to execute when the event is triggered.
	 * @returns {EventUnsubscribe} - A function to unsubscribe from the event.
	 * @see {@link https://rivet.dev/docs/events|Events Documentation}
	 */
	once<Args extends Array<unknown> = unknown[]>(
		eventName: string,
		callback: (...args: Args) => void,
	): EventUnsubscribe {
		return this.#addEventSubscription<Args>(eventName, callback, true);
	}

	/**
	 * Subscribes to connection errors.
	 *
	 * @param {ActorErrorCallback} callback - The callback function to execute when a connection error occurs.
	 * @returns {() => void} - A function to unsubscribe from the error handler.
	 */
	onError(callback: ActorErrorCallback): () => void {
		this.#errorHandlers.add(callback);

		// Return unsubscribe function
		return () => {
			this.#errorHandlers.delete(callback);
		};
	}

	#sendMessage(
		message: {
			body:
				| {
						tag: "ActionRequest";
						val: { id: bigint; name: string; args: unknown };
				  }
				| {
						tag: "SubscriptionRequest";
						val: { eventName: string; subscribe: boolean };
				  };
		},
		opts?: SendHttpMessageOpts,
	) {
		if (this.#disposed) {
			throw new errors.ActorConnDisposed();
		}

		let queueMessage = false;
		if (this.#websocket) {
			const readyState = this.#websocket.readyState;
			logger().debug({
				msg: "websocket send attempt",
				readyState,
				readyStateString:
					readyState === 0
						? "CONNECTING"
						: readyState === 1
							? "OPEN"
							: readyState === 2
								? "CLOSING"
								: "CLOSED",
				connectionId: this.#connectionId,
				messageType: (message.body as any).tag,
				actionName: (message.body as any).val?.name,
			});
			if (readyState === 1) {
				try {
					const messageSerialized = serializeWithEncoding(
						this.#encoding,
						message,
						TO_SERVER_VERSIONED,
						ToServerSchema,
						// JSON: args is the raw value
						(msg): ToServerJson => msg as ToServerJson,
						// BARE: args needs to be CBOR-encoded to ArrayBuffer
						(msg): protocol.ToServer => {
							if (msg.body.tag === "ActionRequest") {
								return {
									body: {
										tag: "ActionRequest",
										val: {
											id: msg.body.val.id,
											name: msg.body.val.name,
											args: bufferToArrayBuffer(
												cbor.encode(msg.body.val.args),
											),
										},
									},
								};
							} else {
								return msg as protocol.ToServer;
							}
						},
					);
					this.#websocket.send(messageSerialized);
					logger().trace({
						msg: "sent websocket message",
						len: messageLength(messageSerialized),
					});
				} catch (error) {
					logger().warn({
						msg: "failed to send message, added to queue",
						error,
						connectionId: this.#connectionId,
					});

					// Assuming the socket is disconnected and will be reconnected soon
					queueMessage = true;
				}
			} else {
				logger().debug({
					msg: "websocket not open, queueing message",
					readyState,
				});
				queueMessage = true;
			}
		} else {
			// No websocket connected yet
			logger().debug({ msg: "no websocket, queueing message" });
			queueMessage = true;
		}

		if (!opts?.ephemeral && queueMessage) {
			this.#messageQueue.push(message);
			logger().debug({
				msg: "queued connection message",
				queueLength: this.#messageQueue.length,
				connectionId: this.#connectionId,
				messageType: (message.body as any).tag,
				actionName: (message.body as any).val?.name,
			});
		}
	}

	async #parseMessage(data: ConnMessage): Promise<{
		body:
			| { tag: "Init"; val: { actorId: string; connectionId: string } }
			| {
					tag: "Error";
					val: {
						group: string;
						code: string;
						message: string;
						metadata: unknown;
						actionId: bigint | null;
					};
			  }
			| { tag: "ActionResponse"; val: { id: bigint; output: unknown } }
			| { tag: "Event"; val: { name: string; args: unknown } };
	}> {
		invariant(this.#websocket, "websocket must be defined");

		const buffer = await inputDataToBuffer(data);

		return deserializeWithEncoding(
			this.#encoding,
			buffer,
			TO_CLIENT_VERSIONED,
			ToClientSchema,
			// JSON: values are already the correct type
			(msg): ToClientJson => msg as ToClientJson,
			// BARE: need to decode ArrayBuffer fields back to unknown
			(msg): any => {
				if (msg.body.tag === "Error") {
					return {
						body: {
							tag: "Error",
							val: {
								group: msg.body.val.group,
								code: msg.body.val.code,
								message: msg.body.val.message,
								metadata: msg.body.val.metadata
									? cbor.decode(
											new Uint8Array(
												msg.body.val.metadata,
											),
										)
									: null,
								actionId: msg.body.val.actionId,
							},
						},
					};
				} else if (msg.body.tag === "ActionResponse") {
					return {
						body: {
							tag: "ActionResponse",
							val: {
								id: msg.body.val.id,
								output: cbor.decode(
									new Uint8Array(msg.body.val.output),
								),
							},
						},
					};
				} else if (msg.body.tag === "Event") {
					return {
						body: {
							tag: "Event",
							val: {
								name: msg.body.val.name,
								args: cbor.decode(
									new Uint8Array(msg.body.val.args),
								),
							},
						},
					};
				} else {
					// Init has no ArrayBuffer fields
					return msg;
				}
			},
		);
	}

	/**
	 * Get the actor ID (for testing purposes).
	 * @internal
	 */
	get actorId(): string | undefined {
		return this.#actorId;
	}

	/**
	 * Get the connection ID (for testing purposes).
	 * @internal
	 */
	get connectionId(): string | undefined {
		return this.#connectionId;
	}

	/**
	 * Disconnects from the actor.
	 *
	 * @returns {Promise<void>} A promise that resolves when the socket is gracefully closed.
	 */
	async dispose(): Promise<void> {
		// Internally, this "disposes" the connection

		if (this.#disposed) {
			logger().warn({ msg: "connection already disconnected" });
			return;
		}
		this.#disposed = true;

		logger().debug({ msg: "disposing actor conn" });

		// Clear interval so NodeJS process can exit
		clearInterval(this.#keepNodeAliveInterval);

		// Abort
		this.#abortController.abort();

		// Remove from registry
		this.#client[ACTOR_CONNS_SYMBOL].delete(this);

		// Disconnect websocket cleanly
		if (this.#websocket) {
			logger().debug("closing ws");

			const ws = this.#websocket;
			// Check if WebSocket is already closed or closing
			if (
				ws.readyState === 2 /* CLOSING */ ||
				ws.readyState === 3 /* CLOSED */
			) {
				logger().debug({ msg: "ws already closed or closing" });
			} else {
				const { promise, resolve } = promiseWithResolvers();
				ws.addEventListener("close", () => {
					logger().debug({ msg: "ws closed" });
					resolve(undefined);
				});
				ws.close(1000, "Normal closure");
				await promise;
			}
		}
		this.#websocket = undefined;
	}

	#sendSubscription(eventName: string, subscribe: boolean) {
		this.#sendMessage(
			{
				body: {
					tag: "SubscriptionRequest",
					val: {
						eventName,
						subscribe,
					},
				},
			},
			{ ephemeral: true },
		);
	}
}

/**
 * Connection to a actor. Allows calling actor's remote procedure calls with inferred types. See {@link ActorConnRaw} for underlying methods.
 *
 * @example
 * ```
 * const room = client.connect<ChatRoom>(...etc...);
 * // This calls the action named `sendMessage` on the `ChatRoom` actor.
 * await room.sendMessage('Hello, world!');
 * ```
 *
 * Private methods (e.g. those starting with `_`) are automatically excluded.
 *
 * @template AD The actor class that this connection is for.
 * @see {@link ActorConnRaw}
 */
export type ActorConn<AD extends AnyActorDefinition> = ActorConnRaw &
	ActorDefinitionActions<AD>;
