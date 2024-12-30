import { assertEquals } from "@std/assert";
import * as cbor from "@std/cbor";
import { MAX_CONN_PARAMS_SIZE } from "../common/network.ts";
import { assertUnreachable } from "../common/utils.ts";
import type { ProtocolFormat } from "../protocol/ws/mod.ts";
import type * as wsToClient from "../protocol/ws/to_client.ts";
import type * as wsToServer from "../protocol/ws/to_server.ts";
import * as errors from "./errors.ts";
import { logger } from "./log.ts";

interface RpcInFlight {
	resolve: (response: wsToClient.RpcResponseOk) => void;
	reject: (error: Error) => void;
}

interface EventSubscriptions<Args extends Array<unknown>> {
	callback: (...args: Args) => void;
	once: boolean;
}

type EventUnsubscribe = () => void;

interface SendOpts {
	ephemeral: boolean;
}

type WebSocketMessage = string | Blob | ArrayBuffer;

export class ActorHandleRaw {
	#disconnected = false;

	#websocket?: WebSocket;
	#websocketQueue: WebSocketMessage[] = [];
	#websocketRpcInFlight = new Map<number, RpcInFlight>();

	// biome-ignore lint/suspicious/noExplicitAny: Unknown subscription type
	#eventSubscriptions = new Map<string, Set<EventSubscriptions<any[]>>>();

	#requestIdCounter = 0;

	// TODO: ws message queue

	constructor(
		private readonly endpoint: string,
		private readonly parameters: unknown,
		private readonly protocolFormat: ProtocolFormat,
	) {}

	async rpc<Args extends Array<unknown> = unknown[], Response = unknown>(
		name: string,
		...args: Args
	): Promise<Response> {
		logger().debug("rpc", { name, args });

		// TODO: Add to queue if socket is not open

		const requestId = this.#requestIdCounter;
		this.#requestIdCounter += 1;

		const {
			promise: resolvePromise,
			resolve,
			reject,
		} = Promise.withResolvers<wsToClient.RpcResponseOk>();
		this.#websocketRpcInFlight.set(requestId, { resolve, reject });

		this.#webSocketSend({
			body: {
				rr: {
					i: requestId,
					n: name,
					a: args,
				},
			},
		} satisfies wsToServer.ToServer);

		// TODO: Throw error if disconnect is called

		const { i: responseId, o: output } = await resolvePromise;
		assertEquals(responseId, requestId);

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

	connect() {
		this.#disconnected = false;

		let url = `${this.endpoint}/connect?version=1&format=${this.protocolFormat}`;

		if (this.parameters !== undefined) {
			const paramsStr = JSON.stringify(this.parameters);

			// TODO: This is an imprecise count since it doesn't count the full URL length & URI encoding expansion in the URL size
			if (paramsStr.length > MAX_CONN_PARAMS_SIZE) {
				throw new errors.ConnectionParametersTooLong();
			}

			url += `&params=${encodeURIComponent(paramsStr)}`;
		}

		const ws = new WebSocket(url);
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
			logger().debug("socket error", { event });
		};
		ws.onmessage = async (ev) => {
			const response = (await this.#parse(ev.data)) as wsToClient.ToClient;

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

				logger().info("received error from actor", {
					rpc: rpcId,
					code,
					message,
					metadata,
				});

				const inFlight = this.#takeRpcInFlight(rpcId);
				inFlight.reject(new errors.RpcError(code, message, metadata));
			} else if ("ev" in response.body) {
				this.#dispatchEvent(response.body.ev);
			} else if ("er" in response.body) {
				const { c: code, m: message, md: metadata } = response.body.er;

				logger().info("received error from actor", {
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
		if (!inFlight)
			throw new errors.InternalError(`No in flight response for ${id}`);
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

	on<Args extends Array<unknown> = unknown[]>(
		eventName: string,
		callback: (...args: Args) => void,
	): EventUnsubscribe {
		return this.#addEventSubscription<Args>(eventName, callback, false);
	}

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
		} else if (this.protocolFormat === "cbor") {
			if (data instanceof Blob) {
				return cbor.decodeCbor(await data.bytes());
			} else if (data instanceof ArrayBuffer) {
				return cbor.decodeCbor(new Uint8Array(data));
			} else {
				throw new Error("received non-binary type for cbor parse");
			}
		} else {
			assertUnreachable(this.protocolFormat);
		}
	}

	#serialize(value: unknown): WebSocketMessage {
		if (this.protocolFormat === "json") {
			return JSON.stringify(value);
		} else if (this.protocolFormat === "cbor") {
			return cbor.encodeCbor(value as cbor.CborType);
		} else {
			assertUnreachable(this.protocolFormat);
		}
	}

	#webSocketSendRaw(message: WebSocketMessage, opts?: SendOpts) {
		if (this.#websocket?.readyState === WebSocket.OPEN) {
			try {
				this.#websocket.send(message);
				logger().debug("sent websocket message", { message });
			} catch (error) {
				logger().warn("failed to send message, added to queue", { error });

				// Assuming the socket is disconnected and will be reconnected soon
				//
				// Will attempt to resend soon
				this.#websocketQueue.unshift(message);
			}
		} else {
			if (!opts?.ephemeral) {
				this.#websocketQueue.push(message);
				logger().debug("queued websocket message", { message });
			}
		}
	}

	// TODO:Add destructor
	disconnect(): Promise<void> {
		return new Promise(resolve => {
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

	async dispose() {
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
