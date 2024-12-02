import { assertExists, assertEquals } from "@std/assert";
import { assertUnreachable } from "../../common/src/utils.ts";
import * as wsToServer from "../../protocol/src/ws/to_server.ts";
import * as wsToClient from "../../protocol/src/ws/to_client.ts";

interface RpcInFlight {
	resolve: (response: wsToClient.RpcResponseOk) => void,
	reject: (error: Error) => void,
}

interface EventSubscriptions {
	callback: (...args: unknown[]) => void;
	once: boolean;
}

type EventUnsubscribe = () => void;

export class ActorHandle {
	#destroyed = false;

	#websocket?: WebSocket;
	#websocketQueue: string[] = [];
	#websocketRpcInFlight = new Map<string, RpcInFlight>();

	#eventSubscriptions = new Map<string, Set<EventSubscriptions>>();

	// TODO: ws message queue

	constructor(private readonly endpoint: string) {
	}

	#assertUsable() {
		if (this.#destroyed) throw new Error("Actor destroyed");
	}

	async rpc<Args extends Array<unknown> = unknown[], Response = unknown>(name: string, ...args: Args): Promise<Response> {
		assertExists(this.#websocket);

		// TODO: Add to queue if socket is not open

		const requestId = crypto.randomUUID();

		const resolvePromise = new Promise<wsToClient.RpcResponseOk>((resolve, reject) => {
			this.#websocketRpcInFlight.set(requestId, { resolve, reject });
		});

		this.#webSocketSend({
			body: {
				rpcRequest: {
					id: requestId,
					name,
					args,
				}
			}
		} satisfies wsToServer.ToServer);

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

	connect<Args extends Array<unknown>>(...args: Args) {
		const url = `${this.endpoint}/connect?args=${encodeURIComponent(JSON.stringify(args))}`;

		const ws = new WebSocket(url);
		this.#websocket = ws;
		ws.onopen = () => {
			console.log("Socket open");

			// Resubscribe to all active events
			for (const eventName of this.#eventSubscriptions.keys()) {
				this.#sendSubscription(eventName, true);
			}

			// Flush queue
			while (this.#websocketQueue.length > 0) {
				const message = this.#websocketQueue.shift();
				assertExists(message);
				ws.send(message);
			}
		};
		ws.onclose = () => {
			// TODO: Handle queue
			// TODO: Reconnect with backoff

			console.log("Socket closed");
			this.#websocket = undefined;

			// Automatically reconnect
			if (!this.#destroyed) {
				// TODO: Fetch actor to check if it's destroyed

				// TODO: Add backoff for reconnect

				// TODO: Add a way of preserving connection ID for connection state

				// this.connect(...args);
			}
		};
		ws.onerror = (ev) => {
			if (this.#destroyed) return;
			console.warn("Socket error", ev);

		};
		ws.onmessage = (ev) => {
			const rawData = ev.data;
			if (typeof rawData !== "string") throw new Error("Response data was not string");

			const response: wsToClient.ToClient = JSON.parse(rawData);

			if ("rpcResponseOk" in response.body) {
				const inFlight = this.#takeRpcInFlight(response.body.rpcResponseOk.id);
				inFlight.resolve(response.body.rpcResponseOk);
			} else if ("rpcResponseError" in response.body) {
				const inFlight = this.#takeRpcInFlight(response.body.rpcResponseError.id);
				inFlight.reject(new Error(`RPC error: ${response.body.rpcResponseError.message}`));
			} else if ("event" in response.body) {
				this.#dispatchEvent(response.body.event);
			} else if ("error" in response.body) {
				console.warn(`Unhandled error from actor: ${response.body.error.message}`);;
			} else {
				assertUnreachable(response.body);
			}
		};
	}

	#takeRpcInFlight(id: string): RpcInFlight {
		const inFlight = this.#websocketRpcInFlight.get(id)
		if (!inFlight) throw new Error(`No in flight response for ${id}`);
		this.#websocketRpcInFlight.delete(id);
		return inFlight;
	}

	#dispatchEvent(event: wsToClient.ToClientEvent) {
		const listeners = this.#eventSubscriptions.get(event.name);
		if (!listeners) return;

		// Create a new array to avoid issues with listeners being removed during iteration
		[...listeners].forEach(listener => {
			listener.callback(...event.args);
			
			// Remove if this was a one-time listener
			if (listener.once) {
				listeners.delete(listener);
			}
		});

		// Clean up empty listener sets
		if (listeners.size === 0) {
			this.#eventSubscriptions.delete(event.name);
		}
	}

	#addEventSubscription(eventName: string, callback: (...args: unknown[]) => void, once: boolean): EventUnsubscribe {
		this.#assertUsable();

		const listener: EventSubscriptions = {
			callback,
			once
		};

		const isFirstListener = !this.#eventSubscriptions.has(eventName);
		if (isFirstListener) {
			this.#eventSubscriptions.set(eventName, new Set());
			this.#sendSubscription(eventName, true);
		}
		this.#eventSubscriptions.get(eventName)!.add(listener);

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

	on(eventName: string, callback: (...args: unknown[]) => void): EventUnsubscribe {
		return this.#addEventSubscription(eventName, callback, false);
	}

	once(eventName: string, callback: (...args: unknown[]) => void): EventUnsubscribe {
		return this.#addEventSubscription(eventName, callback, true);
	}

	#webSocketSend(message: wsToServer.ToServer) {
		const msgJson = JSON.stringify(message);
		if (this.#websocket?.readyState == WebSocket.OPEN) {
			try {
				this.#websocket.send(msgJson);
			} catch (err) {
				console.warn("Failed to send message, retrying", err);

				// Assuming the socket is disconnected and will be reconnected soon
				//
				// Will attempt to resend soon
				this.#websocketQueue.unshift(msgJson);
			}
		} else {
			this.#websocketQueue.push(msgJson);
		}
	}

	// TODO:Add destructor
	disconnect() {
		if (!this.#websocket) return;

		console.log("Disconnecting");

		// TODO: What do we do with the queue?

		this.#websocket?.close();
		this.#websocket = undefined;
	}

	dispose() {
		console.log("Disposing");

		// TODO: this will error if not usable
		this.disconnect();
	}

	#sendSubscription(eventName: string, subscribe: boolean) {
		const msgJson = JSON.stringify({
			body: {
				subscriptionRequest: {
					eventName,
					subscribe,
				}
			}
		} satisfies wsToServer.ToServer);

		if (this.#websocket?.readyState == WebSocket.OPEN) {
			try {
				this.#websocket.send(msgJson);
			} catch (err) {
				console.warn("Failed to send subscription message, retrying", err);

				// Assuming the socket is disconnected and will be reconnected soon
				//
				// Will attempt to resend soon
				this.#websocketQueue.unshift(msgJson);
			}
		} else {
			this.#websocketQueue.push(msgJson);
		}
	}
}

