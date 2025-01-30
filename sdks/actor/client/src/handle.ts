import { MAX_CONN_PARAMS_SIZE } from "@rivet-gg/actor-common/network";
import { assertUnreachable } from "@rivet-gg/actor-common/utils";
import type { ProtocolFormat } from "@rivet-gg/actor-protocol/ws";
import type * as wsToClient from "@rivet-gg/actor-protocol/ws/to_client";
import type * as wsToServer from "@rivet-gg/actor-protocol/ws/to_server";
import * as cbor from "cbor-x";
import * as errors from "./errors";
import { logger } from "./log";
import { type WebSocketMessage, messageLength } from "./utils";

interface RpcInFlight {
	name: string;
	resolve: (response: wsToClient.RpcResponseOk) => void;
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

interface SendOpts {
	ephemeral: boolean;
}

/**
 * Provides underlying functions for {@link ActorHandle}. See {@link ActorHandle} for using type-safe remote procedure calls.
 *
 * @see {@link ActorHandle}
 */
export class ActorHandleRaw {
	#disconnected = false;

	#websocket?: WebSocket;
	#websocketQueue: WebSocketMessage[] = [];
	#websocketRpcInFlight = new Map<number, RpcInFlight>();

	// biome-ignore lint/suspicious/noExplicitAny: Unknown subscription type
	#eventSubscriptions = new Map<string, Set<EventSubscriptions<any[]>>>();

	#rpcIdCounter = 0;

	// TODO: ws message queue

	/**
	 * Do not call this directly.
	 *
	 * Creates an instance of ActorHandleRaw.
	 *
	 * @param {string} endpoint - The endpoint to connect to.
	 * @param {unknown} parameters - The parameters to pass to the connection.
	 * @param {ProtocolFormat} protocolFormat - The format used for protocol communication.
	 *
	 * @protected
	 */
	public constructor(
		private readonly endpoint: string,
		private readonly parameters: unknown,
		private readonly protocolFormat: ProtocolFormat,
	) {}

	/**
	 * Call a raw RPC handle. See {@link ActorHandle} for type-safe RPC calls.
	 *
	 * @see {@link ActorHandle}
	 * @template Args - The type of arguments to pass to the RPC function.
	 * @template Response - The type of the response returned by the RPC function.
	 * @param {string} name - The name of the RPC function to call.
	 * @param {...Args} args - The arguments to pass to the RPC function.
	 * @returns {Promise<Response>} - A promise that resolves to the response of the RPC function.
	 */
	async rpc<Args extends Array<unknown> = unknown[], Response = unknown>(
		name: string,
		...args: Args
	): Promise<Response> {
		logger().debug("rpc", { name, args });

		// TODO: Add to queue if socket is not open

		const rpcId = this.#rpcIdCounter;
		this.#rpcIdCounter += 1;

		const { promise, resolve, reject } =
			Promise.withResolvers<wsToClient.RpcResponseOk>();
		this.#websocketRpcInFlight.set(rpcId, { name, resolve, reject });

		this.#webSocketSend({
			body: {
				rr: {
					i: rpcId,
					n: name,
					a: args,
				},
			},
		} satisfies wsToServer.ToServer);

		// TODO: Throw error if disconnect is called

		const { i: responseId, o: output } = await promise;
		if (responseId !== rpcId)
			throw new Error(
				`Request ID ${rpcId} does not match response ID ${responseId}`,
			);

		return output as Response;
	}

	//async #rpcHttp<Args extends Array<unknown> = unknown[], Response = unknown>(name: string, ...args: Args): Promise<Response> {
	//	const origin = `${resolved.isTls ? "https": "http"}://${resolved.publicHostname}:${resolved.publicPort}`;
	//	const url = `${origin}/rpc/${encodeURIComponent(name)}`;
	//	const res = await fetch(url, {
	//		method: "POST",
	//		// TODO: Import type from protocol
	//		body: JSON.stringify({
	//			args,
	//		})
	//	});
	//	if (!res.ok) {
	//		throw new Error(`RPC error (${res.statusText}):\n${await res.text()}`);
	//	}
	//	// TODO: Import type from protocol
	//	const resJson: httpRpc.ResponseOk<Response> = await res.json();
	//	return resJson.output;
	//}

	/**
	 * Do not call this directly.
	 *
	 * Establishes a WebSocket connection to the server using the specified endpoint and protocol format.
	 *
	 * @protected
	 */
	public connect() {
		this.#disconnected = false;

		let url = `${this.endpoint}/connect?format=${this.protocolFormat}`;

		if (this.parameters !== undefined) {
			const paramsStr = JSON.stringify(this.parameters);

			// TODO: This is an imprecise count since it doesn't count the full URL length & URI encoding expansion in the URL size
			if (paramsStr.length > MAX_CONN_PARAMS_SIZE) {
				throw new errors.ConnectionParametersTooLong();
			}

			url += `&params=${encodeURIComponent(paramsStr)}`;
		}

		const ws = new WebSocket(url);
		ws.binaryType = this.protocolFormat === "cbor" ? "arraybuffer" : "blob";
		this.#websocket = ws;
		ws.onopen = () => {
			logger().debug("socket open");

			// Resubscribe to all active events
			for (const eventName of this.#eventSubscriptions.keys()) {
				this.#sendSubscription(eventName, true);
			}

			// Flush queue
			//
			// If the message fails to send, the message will be re-queued
			const queue = this.#websocketQueue;
			this.#websocketQueue = [];
			for (const msg of queue) {
				this.#webSocketSendRaw(msg);
			}
		};
		ws.onclose = (ev) => {
			// TODO: Handle queue
			// TODO: Reconnect with backoff

			logger().debug("socket closed", {
				code: ev.code,
				reason: ev.reason,
				wasClean: ev.wasClean,
			});
			this.#websocket = undefined;

			// Automatically reconnect
			if (!this.#disconnected) {
				// TODO: Fetch actor to check if it's destroyed
				// TODO: Add backoff for reconnect
				// TODO: Add a way of preserving connection ID for connection state
				// this.connect(...args);
			}
		};
		ws.onerror = (event) => {
			if (this.#disconnected) return;
			logger().warn("socket error", { event });
		};
		ws.onmessage = async (ev) => {
			const response = (await this.#parse(
				ev.data,
			)) as wsToClient.ToClient;

			if ("ro" in response.body) {
				// RPC response OK

				const { i: rpcId } = response.body.ro;

				const inFlight = this.#takeRpcInFlight(rpcId);
				inFlight.resolve(response.body.ro);
			} else if ("re" in response.body) {
				// RPC response error

				const {
					i: rpcId,
					c: code,
					m: message,
					md: metadata,
				} = response.body.re;

				const inFlight = this.#takeRpcInFlight(rpcId);

				logger().warn("actor error", {
					rpcId,
					rpcName: inFlight?.name,
					code,
					message,
					metadata,
				});

				inFlight.reject(new errors.RpcError(code, message, metadata));
			} else if ("ev" in response.body) {
				this.#dispatchEvent(response.body.ev);
			} else if ("er" in response.body) {
				const { c: code, m: message, md: metadata } = response.body.er;

				logger().warn("actor error", {
					code,
					message,
					metadata,
				});
			} else {
				assertUnreachable(response.body);
			}
		};
	}

	#takeRpcInFlight(id: number): RpcInFlight {
		const inFlight = this.#websocketRpcInFlight.get(id);
		if (!inFlight) {
			throw new errors.InternalError(`No in flight response for ${id}`);
		}
		this.#websocketRpcInFlight.delete(id);
		return inFlight;
	}

	#dispatchEvent(event: wsToClient.ToClientEvent) {
		const { n: name, a: args } = event;

		const listeners = this.#eventSubscriptions.get(name);
		if (!listeners) return;

		// Create a new array to avoid issues with listeners being removed during iteration
		for (const listener of [...listeners]) {
			listener.callback(...args);

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
	 * @see {@link https://rivet.gg/docs/events|Events Documentation}
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
	 * @see {@link https://rivet.gg/docs/events|Events Documentation}
	 */
	once<Args extends Array<unknown> = unknown[]>(
		eventName: string,
		callback: (...args: Args) => void,
	): EventUnsubscribe {
		return this.#addEventSubscription<Args>(eventName, callback, true);
	}

	#webSocketSend(message: wsToServer.ToServer, opts?: SendOpts) {
		this.#webSocketSendRaw(this.#serialize(message), opts);
	}

	async #parse(data: WebSocketMessage): Promise<unknown> {
		if (this.protocolFormat === "json") {
			if (typeof data !== "string") {
				throw new Error("received non-string for json parse");
			}
			return JSON.parse(data);
		}
		if (this.protocolFormat === "cbor") {
			if (data instanceof Blob) {
				return cbor.decode(new Uint8Array(await data.arrayBuffer()));
			}
			if (data instanceof ArrayBuffer) {
				return cbor.decode(new Uint8Array(data));
			}
			throw new Error("received non-binary type for cbor parse");
		}
		assertUnreachable(this.protocolFormat);
	}

	#serialize(value: unknown): WebSocketMessage {
		if (this.protocolFormat === "json") {
			return JSON.stringify(value);
		}
		if (this.protocolFormat === "cbor") {
			return cbor.encode(value);
		}
		assertUnreachable(this.protocolFormat);
	}

	#webSocketSendRaw(message: WebSocketMessage, opts?: SendOpts) {
		if (this.#websocket?.readyState === WebSocket.OPEN) {
			try {
				this.#websocket.send(message);
				logger().debug("sent websocket message", {
					len: messageLength(message),
				});
			} catch (error) {
				logger().warn("failed to send message, added to queue", {
					error,
				});

				// Assuming the socket is disconnected and will be reconnected soon
				//
				// Will attempt to resend soon
				this.#websocketQueue.unshift(message);
			}
		} else {
			if (!opts?.ephemeral) {
				this.#websocketQueue.push(message);
				logger().debug("queued websocket message", {
					len: messageLength(message),
				});
			}
		}
	}

	// TODO: Add destructor

	/**
	 * Disconnects the WebSocket connection.
	 *
	 * @returns {Promise<void>} A promise that resolves when the WebSocket connection is closed.
	 */
	disconnect(): Promise<void> {
		return new Promise((resolve) => {
			if (!this.#websocket) return;
			this.#disconnected = true;

			logger().debug("disconnecting");

			// TODO: What do we do with the queue?

			if (this.#websocket) {
				this.#websocket.addEventListener("close", () => resolve());
				this.#websocket.close();
				this.#websocket = undefined;
			}
		});
	}

	/**
	 * Disposes of the ActorHandleRaw instance by disconnecting the WebSocket connection.
	 *
	 * @returns {Promise<void>} A promise that resolves when the WebSocket connection is closed.
	 */
	async dispose(): Promise<void> {
		logger().debug("disposing");

		// TODO: this will error if not usable
		await this.disconnect();
	}

	#sendSubscription(eventName: string, subscribe: boolean) {
		this.#webSocketSend(
			{
				body: {
					sr: {
						e: eventName,
						s: subscribe,
					},
				},
			},
			{ ephemeral: true },
		);
	}
}
