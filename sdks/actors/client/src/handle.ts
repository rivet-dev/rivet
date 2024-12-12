import { assertEquals } from "@std/assert";
import { MAX_CONN_PARAMS_SIZE } from "../../common/src/network.ts";
import { assertUnreachable } from "../../common/src/utils.ts";
import type * as wsToClient from "../../protocol/src/ws/to_client.ts";
import type * as wsToServer from "../../protocol/src/ws/to_server.ts";
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

export class ActorHandleRaw {
	#disconnected = false;

	#websocket?: WebSocket;
	#websocketQueue: string[] = [];
	#websocketRpcInFlight = new Map<string, RpcInFlight>();

	// biome-ignore lint/suspicious/noExplicitAny: Unknown subscription type
	#eventSubscriptions = new Map<string, Set<EventSubscriptions<any[]>>>();

	// TODO: ws message queue

	constructor(
		private readonly endpoint: string,
		private readonly parameters: unknown,
	) {}

	async rpc<Args extends Array<unknown> = unknown[], Response = unknown>(
		name: string,
		...args: Args
	): Promise<Response> {
		logger().debug("rpc", { name, args });

		// TODO: Add to queue if socket is not open

		const requestId = crypto.randomUUID();

		const resolvePromise = new Promise<wsToClient.RpcResponseOk>(
			(resolve, reject) => {
				this.#websocketRpcInFlight.set(requestId, { resolve, reject });
			},
		);

		this.#webSocketSend(
			{
				body: {
					rpcRequest: {
						id: requestId,
						name,
						args,
					},
				},
			} satisfies wsToServer.ToServer,
		);

		// TODO: Throw error if disconnect is called

		const res = await resolvePromise;
		assertEquals(res.id, requestId);

		return res.output as Response;
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

		let url = `${this.endpoint}/connect?version=1`;

		if (this.parameters !== undefined) {
			const paramsStr = JSON.stringify(this.parameters);

			// TODO: This is an imprecise count since it doesn't count the full URL length & URI encoding expansion in the URL size
			if (paramsStr.length > MAX_CONN_PARAMS_SIZE) {
				throw new Error(
					`Connection parameters must be less than ${MAX_CONN_PARAMS_SIZE} bytes`,
				);
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
		ws.onmessage = (ev) => {
			const rawData = ev.data;
			if (typeof rawData !== "string") {
				throw new Error("Response data was not string");
			}

			const response: wsToClient.ToClient = JSON.parse(rawData);

			if ("rpcResponseOk" in response.body) {
				const inFlight = this.#takeRpcInFlight(response.body.rpcResponseOk.id);
				inFlight.resolve(response.body.rpcResponseOk);
			} else if ("rpcResponseError" in response.body) {
				const inFlight = this.#takeRpcInFlight(
					response.body.rpcResponseError.id,
				);
				inFlight.reject(
					new Error(`RPC error: ${response.body.rpcResponseError.message}`),
				);
			} else if ("event" in response.body) {
				this.#dispatchEvent(response.body.event);
			} else if ("error" in response.body) {
				logger().warn(
					"unhandled error from actor",
					{ message: response.body.error.message },
				);
			} else {
				assertUnreachable(response.body);
			}
		};
	}

	#takeRpcInFlight(id: string): RpcInFlight {
		const inFlight = this.#websocketRpcInFlight.get(id);
		if (!inFlight) throw new Error(`No in flight response for ${id}`);
		this.#websocketRpcInFlight.delete(id);
		return inFlight;
	}

	#dispatchEvent(event: wsToClient.ToClientEvent) {
		const listeners = this.#eventSubscriptions.get(event.name);
		if (!listeners) return;

		// Create a new array to avoid issues with listeners being removed during iteration
		for (const listener of [...listeners]) {
			listener.callback(...event.args);

			// Remove if this was a one-time listener
			if (listener.once) {
				listeners.delete(listener);
			}
		}

		// Clean up empty listener sets
		if (listeners.size === 0) {
			this.#eventSubscriptions.delete(event.name);
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
		this.#webSocketSendRaw(JSON.stringify(message), opts);
	}

	#webSocketSendRaw(message: string, opts?: SendOpts) {
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
	disconnect() {
		if (!this.#websocket) return;
		this.#disconnected = true;

		logger().debug("disconnecting");

		// TODO: What do we do with the queue?

		this.#websocket?.close();
		this.#websocket = undefined;
	}

	dispose() {
		logger().debug("disposing");

		// TODO: this will error if not usable
		this.disconnect();
	}

	#sendSubscription(eventName: string, subscribe: boolean) {
		this.#webSocketSend(
			{
				body: {
					subscriptionRequest: {
						eventName,
						subscribe,
					},
				},
			},
			{ ephemeral: true },
		);
	}
}
