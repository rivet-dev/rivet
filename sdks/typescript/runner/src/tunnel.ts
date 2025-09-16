import WebSocket from "ws";
import * as tunnel from "@rivetkit/engine-tunnel-protocol";
import { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter";
import { calculateBackoff } from "./utils";
import type { Runner, ActorInstance } from "./mod";
import { v4 as uuidv4 } from "uuid";
import { logger } from "./log";

const GC_INTERVAL = 60000; // 60 seconds
const MESSAGE_ACK_TIMEOUT = 5000; // 5 seconds

interface PendingRequest {
	resolve: (response: Response) => void;
	reject: (error: Error) => void;
	streamController?: ReadableStreamDefaultController<Uint8Array>;
	actorId?: string;
}

interface TunnelCallbacks {
	onConnected(): void;
	onDisconnected(): void;
}

interface PendingMessage {
	sentAt: number;
	requestIdStr: string;
}

export class Tunnel {
	#pegboardTunnelUrl: string;

	#runner: Runner;

	#tunnelWs?: WebSocket;
	#shutdown = false;
	#reconnectTimeout?: NodeJS.Timeout;
	#reconnectAttempt = 0;

	#actorPendingRequests: Map<string, PendingRequest> = new Map();
	#actorWebSockets: Map<string, WebSocketTunnelAdapter> = new Map();

	#pendingMessages: Map<string, PendingMessage> = new Map();
	#gcInterval?: NodeJS.Timeout;

	#callbacks: TunnelCallbacks;

	constructor(
		runner: Runner,
		pegboardTunnelUrl: string,
		callbacks: TunnelCallbacks,
	) {
		this.#pegboardTunnelUrl = pegboardTunnelUrl;
		this.#runner = runner;
		this.#callbacks = callbacks;
	}

	start(): void {
		if (this.#tunnelWs?.readyState === WebSocket.OPEN) {
			return;
		}

		this.#connect();
		this.#startGarbageCollector();
	}

	shutdown() {
		this.#shutdown = true;

		if (this.#reconnectTimeout) {
			clearTimeout(this.#reconnectTimeout);
			this.#reconnectTimeout = undefined;
		}

		if (this.#gcInterval) {
			clearInterval(this.#gcInterval);
			this.#gcInterval = undefined;
		}

		if (this.#tunnelWs) {
			this.#tunnelWs.close();
			this.#tunnelWs = undefined;
		}

		// TODO: Should we use unregisterActor instead

		// Reject all pending requests
		for (const [_, request] of this.#actorPendingRequests) {
			request.reject(new Error("Tunnel shutting down"));
		}
		this.#actorPendingRequests.clear();

		// Close all WebSockets
		for (const [_, ws] of this.#actorWebSockets) {
			ws.close();
		}
		this.#actorWebSockets.clear();
	}

	#sendMessage(requestId: tunnel.RequestId, messageKind: tunnel.MessageKind) {
		if (!this.#tunnelWs || this.#tunnelWs.readyState !== WebSocket.OPEN) {
			console.warn("Cannot send tunnel message, WebSocket not connected");
			return;
		}

		// Build message
		const messageId = generateUuidBuffer();

		const requestIdStr = bufferToString(requestId);
		this.#pendingMessages.set(bufferToString(messageId), {
			sentAt: Date.now(),
			requestIdStr,
		});

		// Send message
		const message: tunnel.RunnerMessage = {
			requestId,
			messageId,
			messageKind,
		};

		const encoded = tunnel.encodeRunnerMessage(message);
		this.#tunnelWs.send(encoded);
	}

	#sendAck(requestId: tunnel.RequestId, messageId: tunnel.MessageId) {
		if (!this.#tunnelWs || this.#tunnelWs.readyState !== WebSocket.OPEN) {
			return;
		}

		const message: tunnel.RunnerMessage = {
			requestId,
			messageId,
			messageKind: { tag: "Ack", val: null },
		};

		const encoded = tunnel.encodeRunnerMessage(message);
		this.#tunnelWs.send(encoded);
	}

	#startGarbageCollector() {
		if (this.#gcInterval) {
			clearInterval(this.#gcInterval);
		}

		this.#gcInterval = setInterval(() => {
			this.#gc();
		}, GC_INTERVAL);
	}

	#gc() {
		const now = Date.now();
		const messagesToDelete: string[] = [];

		for (const [messageId, pendingMessage] of this.#pendingMessages) {
			// Check if message is older than timeout
			if (now - pendingMessage.sentAt > MESSAGE_ACK_TIMEOUT) {
				messagesToDelete.push(messageId);

				const requestIdStr = pendingMessage.requestIdStr;

				// Check if this is an HTTP request
				const pendingRequest =
					this.#actorPendingRequests.get(requestIdStr);
				if (pendingRequest) {
					// Reject the pending HTTP request
					pendingRequest.reject(
						new Error("Message acknowledgment timeout"),
					);

					// Close stream controller if it exists
					if (pendingRequest.streamController) {
						pendingRequest.streamController.error(
							new Error("Message acknowledgment timeout"),
						);
					}

					// Clean up from actorPendingRequests map
					this.#actorPendingRequests.delete(requestIdStr);
				}

				// Check if this is a WebSocket
				const webSocket = this.#actorWebSockets.get(requestIdStr);
				if (webSocket) {
					// Close the WebSocket connection
					webSocket.close(1000, "Message acknowledgment timeout");

					// Clean up from actorWebSockets map
					this.#actorWebSockets.delete(requestIdStr);
				}
			}
		}

		// Remove timed out messages
		for (const messageId of messagesToDelete) {
			this.#pendingMessages.delete(messageId);
			console.warn(`Purged unacked message: ${messageId}`);
		}
	}

	unregisterActor(actor: ActorInstance) {
		const actorId = actor.actorId;

		// Terminate all requests for this actor
		for (const requestId of actor.requests) {
			const pending = this.#actorPendingRequests.get(requestId);
			if (pending) {
				pending.reject(new Error(`Actor ${actorId} stopped`));
				this.#actorPendingRequests.delete(requestId);
			}
		}
		actor.requests.clear();

		// Close all WebSockets for this actor
		for (const webSocketId of actor.webSockets) {
			const ws = this.#actorWebSockets.get(webSocketId);
			if (ws) {
				ws.close(1000, "Actor stopped");
				this.#actorWebSockets.delete(webSocketId);
			}
		}
		actor.webSockets.clear();
	}

	async #fetch(actorId: string, request: Request): Promise<Response> {
		// Validate actor exists
		if (!this.#runner.hasActor(actorId)) {
			logger()?.warn({
				msg: "ignoring request for unknown actor",
				actorId,
			});
			return new Response("Actor not found", { status: 404 });
		}

		const fetchHandler = this.#runner.config.fetch(actorId, request);

		if (!fetchHandler) {
			return new Response("Not Implemented", { status: 501 });
		}

		return fetchHandler;
	}

	#connect() {
		if (this.#shutdown) return;

		try {
			this.#tunnelWs = new WebSocket(this.#pegboardTunnelUrl, {
				headers: {
					"x-rivet-target": "tunnel",
				},
			});

			this.#tunnelWs.binaryType = "arraybuffer";

			this.#tunnelWs.addEventListener("open", () => {
				this.#reconnectAttempt = 0;

				if (this.#reconnectTimeout) {
					clearTimeout(this.#reconnectTimeout);
					this.#reconnectTimeout = undefined;
				}

				this.#callbacks.onConnected();
			});

			this.#tunnelWs.addEventListener("message", async (event) => {
				try {
					await this.#handleMessage(event.data as ArrayBuffer);
				} catch (error) {
					logger()?.error({
						msg: "error handling tunnel message",
						error,
					});
				}
			});

			this.#tunnelWs.addEventListener("error", (event) => {
				logger()?.error({ msg: "tunnel websocket error", event });
			});

			this.#tunnelWs.addEventListener("close", () => {
				this.#callbacks.onDisconnected();

				if (!this.#shutdown) {
					this.#scheduleReconnect();
				}
			});
		} catch (error) {
			logger()?.error({ msg: "failed to connect tunnel", error });
			if (!this.#shutdown) {
				this.#scheduleReconnect();
			}
		}
	}

	#scheduleReconnect() {
		if (this.#shutdown) return;

		const delay = calculateBackoff(this.#reconnectAttempt, {
			initialDelay: 1000,
			maxDelay: 30000,
			multiplier: 2,
			jitter: true,
		});

		this.#reconnectAttempt++;

		this.#reconnectTimeout = setTimeout(() => {
			this.#connect();
		}, delay);
	}

	async #handleMessage(data: ArrayBuffer) {
		const message = tunnel.decodeRunnerMessage(new Uint8Array(data));

		if (message.messageKind.tag === "Ack") {
			// Mark pending message as acknowledged and remove it
			const msgIdStr = bufferToString(message.messageId);
			const pending = this.#pendingMessages.get(msgIdStr);
			if (pending) {
				this.#pendingMessages.delete(msgIdStr);
			}
		} else {
			this.#sendAck(message.requestId, message.messageId);
			switch (message.messageKind.tag) {
				case "ToServerRequestStart":
					await this.#handleRequestStart(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToServerRequestChunk":
					await this.#handleRequestChunk(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToServerRequestAbort":
					await this.#handleRequestAbort(message.requestId);
					break;
				case "ToServerWebSocketOpen":
					await this.#handleWebSocketOpen(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToServerWebSocketMessage":
					await this.#handleWebSocketMessage(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToServerWebSocketClose":
					await this.#handleWebSocketClose(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientResponseStart":
					this.#handleResponseStart(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientResponseChunk":
					this.#handleResponseChunk(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientResponseAbort":
					this.#handleResponseAbort(message.requestId);
					break;
				case "ToClientWebSocketOpen":
					this.#handleWebSocketOpenResponse(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientWebSocketMessage":
					this.#handleWebSocketMessageResponse(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientWebSocketClose":
					this.#handleWebSocketCloseResponse(
						message.requestId,
						message.messageKind.val,
					);
					break;
			}
		}
	}

	async #handleRequestStart(
		requestId: ArrayBuffer,
		req: tunnel.ToServerRequestStart,
	) {
		// Track this request for the actor
		const requestIdStr = bufferToString(requestId);
		const actor = this.#runner.getActor(req.actorId);
		if (actor) {
			actor.requests.add(requestIdStr);
		}

		try {
			// Convert headers map to Headers object
			const headers = new Headers();
			for (const [key, value] of req.headers) {
				headers.append(key, value);
			}

			// Create Request object
			const request = new Request(`http://localhost${req.path}`, {
				method: req.method,
				headers,
				body: req.body ? new Uint8Array(req.body) : undefined,
			});

			// Handle streaming request
			if (req.stream) {
				// Create a stream for the request body
				const stream = new ReadableStream<Uint8Array>({
					start: (controller) => {
						// Store controller for chunks
						const existing =
							this.#actorPendingRequests.get(requestIdStr);
						if (existing) {
							existing.streamController = controller;
							existing.actorId = req.actorId;
						} else {
							this.#actorPendingRequests.set(requestIdStr, {
								resolve: () => {},
								reject: () => {},
								streamController: controller,
								actorId: req.actorId,
							});
						}
					},
				});

				// Create request with streaming body
				const streamingRequest = new Request(request, {
					body: stream,
					duplex: "half",
				} as any);

				// Call fetch handler with validation
				const response = await this.#fetch(
					req.actorId,
					streamingRequest,
				);
				await this.#sendResponse(requestId, response);
			} else {
				// Non-streaming request
				const response = await this.#fetch(req.actorId, request);
				await this.#sendResponse(requestId, response);
			}
		} catch (error) {
			logger()?.error({ msg: "error handling request", error });
			this.#sendResponseError(requestId, 500, "Internal Server Error");
		} finally {
			// Clean up request tracking
			const actor = this.#runner.getActor(req.actorId);
			if (actor) {
				actor.requests.delete(requestIdStr);
			}
		}
	}

	async #handleRequestChunk(
		requestId: ArrayBuffer,
		chunk: tunnel.ToServerRequestChunk,
	) {
		const requestIdStr = bufferToString(requestId);
		const pending = this.#actorPendingRequests.get(requestIdStr);
		if (pending?.streamController) {
			pending.streamController.enqueue(new Uint8Array(chunk.body));
			if (chunk.finish) {
				pending.streamController.close();
				this.#actorPendingRequests.delete(requestIdStr);
			}
		}
	}

	async #handleRequestAbort(requestId: ArrayBuffer) {
		const requestIdStr = bufferToString(requestId);
		const pending = this.#actorPendingRequests.get(requestIdStr);
		if (pending?.streamController) {
			pending.streamController.error(new Error("Request aborted"));
		}
		this.#actorPendingRequests.delete(requestIdStr);
	}

	async #sendResponse(requestId: ArrayBuffer, response: Response) {
		// Always treat responses as non-streaming for now
		// In the future, we could detect streaming responses based on:
		// - Transfer-Encoding: chunked
		// - Content-Type: text/event-stream
		// - Explicit stream flag from the handler

		// Read the body first to get the actual content
		const body = response.body ? await response.arrayBuffer() : null;

		// Convert headers to map and add Content-Length if not present
		const headers = new Map<string, string>();
		response.headers.forEach((value, key) => {
			headers.set(key, value);
		});

		// Add Content-Length header if we have a body and it's not already set
		if (body && !headers.has("content-length")) {
			headers.set("content-length", String(body.byteLength));
		}

		// Send as non-streaming response
		this.#sendMessage(requestId, {
			tag: "ToClientResponseStart",
			val: {
				status: response.status as tunnel.u16,
				headers,
				body: body || null,
				stream: false,
			},
		});
	}

	#sendResponseError(
		requestId: ArrayBuffer,
		status: number,
		message: string,
	) {
		const headers = new Map<string, string>();
		headers.set("content-type", "text/plain");

		this.#sendMessage(requestId, {
			tag: "ToClientResponseStart",
			val: {
				status: status as tunnel.u16,
				headers,
				body: new TextEncoder().encode(message).buffer as ArrayBuffer,
				stream: false,
			},
		});
	}

	async #handleWebSocketOpen(
		requestId: ArrayBuffer,
		open: tunnel.ToServerWebSocketOpen,
	) {
		const webSocketId = bufferToString(requestId);
		// Validate actor exists
		const actor = this.#runner.getActor(open.actorId);
		if (!actor) {
			logger()?.warn({
				msg: "ignoring websocket for unknown actor",
				actorId: open.actorId,
			});
			// Send close immediately
			this.#sendMessage(requestId, {
				tag: "ToClientWebSocketClose",
				val: {
					code: 1011,
					reason: "Actor not found",
				},
			});
			return;
		}

		const websocketHandler = this.#runner.config.websocket;

		if (!websocketHandler) {
			console.error("No websocket handler configured for tunnel");
			logger()?.error({
				msg: "no websocket handler configured for tunnel",
			});
			// Send close immediately
			this.#sendMessage(requestId, {
				tag: "ToClientWebSocketClose",
				val: {
					code: 1011,
					reason: "Not Implemented",
				},
			});
			return;
		}

		// Track this WebSocket for the actor
		if (actor) {
			actor.webSockets.add(webSocketId);
		}

		try {
			// Create WebSocket adapter
			const adapter = new WebSocketTunnelAdapter(
				webSocketId,
				(data: ArrayBuffer | string, isBinary: boolean) => {
					// Send message through tunnel
					const dataBuffer =
						typeof data === "string"
							? (new TextEncoder().encode(data)
									.buffer as ArrayBuffer)
							: data;

					this.#sendMessage(requestId, {
						tag: "ToClientWebSocketMessage",
						val: {
							data: dataBuffer,
							binary: isBinary,
						},
					});
				},
				(code?: number, reason?: string) => {
					// Send close through tunnel
					this.#sendMessage(requestId, {
						tag: "ToClientWebSocketClose",
						val: {
							code: code || null,
							reason: reason || null,
						},
					});

					// Remove from map
					this.#actorWebSockets.delete(webSocketId);

					// Clean up actor tracking
					if (actor) {
						actor.webSockets.delete(webSocketId);
					}
				},
			);

			// Store adapter
			this.#actorWebSockets.set(webSocketId, adapter);

			// Send open confirmation
			this.#sendMessage(requestId, {
				tag: "ToClientWebSocketOpen",
				val: null,
			});

			// Notify adapter that connection is open
			adapter._handleOpen();

			// Create a minimal request object for the websocket handler
			// Include original headers from the open message
			const headerInit: Record<string, string> = {};
			if (open.headers) {
				for (const [k, v] of open.headers as ReadonlyMap<
					string,
					string
				>) {
					headerInit[k] = v;
				}
			}
			// Ensure websocket upgrade headers are present
			headerInit["Upgrade"] = "websocket";
			headerInit["Connection"] = "Upgrade";

			const request = new Request(`http://localhost${open.path}`, {
				method: "GET",
				headers: headerInit,
			});

			// Call websocket handler
			await websocketHandler(open.actorId, adapter, request);
		} catch (error) {
			logger()?.error({ msg: "error handling websocket open", error });
			// Send close on error
			this.#sendMessage(requestId, {
				tag: "ToClientWebSocketClose",
				val: {
					code: 1011,
					reason: "Server Error",
				},
			});

			this.#actorWebSockets.delete(webSocketId);

			// Clean up actor tracking
			if (actor) {
				actor.webSockets.delete(webSocketId);
			}
		}
	}

	async #handleWebSocketMessage(
		requestId: ArrayBuffer,
		msg: tunnel.ToServerWebSocketMessage,
	) {
		const webSocketId = bufferToString(requestId);
		const adapter = this.#actorWebSockets.get(webSocketId);
		if (adapter) {
			const data = msg.binary
				? new Uint8Array(msg.data)
				: new TextDecoder().decode(new Uint8Array(msg.data));

			adapter._handleMessage(data, msg.binary);
		}
	}

	async #handleWebSocketClose(
		requestId: ArrayBuffer,
		close: tunnel.ToServerWebSocketClose,
	) {
		const webSocketId = bufferToString(requestId);
		const adapter = this.#actorWebSockets.get(webSocketId);
		if (adapter) {
			adapter._handleClose(
				close.code || undefined,
				close.reason || undefined,
			);
			this.#actorWebSockets.delete(webSocketId);
		}
	}

	#handleResponseStart(
		requestId: ArrayBuffer,
		resp: tunnel.ToClientResponseStart,
	) {
		const requestIdStr = bufferToString(requestId);
		const pending = this.#actorPendingRequests.get(requestIdStr);
		if (!pending) {
			logger()?.warn({
				msg: "received response for unknown request",
				requestId: requestIdStr,
			});
			return;
		}

		// Convert headers map to Headers object
		const headers = new Headers();
		for (const [key, value] of resp.headers) {
			headers.append(key, value);
		}

		if (resp.stream) {
			// Create streaming response
			const stream = new ReadableStream<Uint8Array>({
				start: (controller) => {
					pending.streamController = controller;
				},
			});

			const response = new Response(stream, {
				status: resp.status,
				headers,
			});

			pending.resolve(response);
		} else {
			// Non-streaming response
			const body = resp.body ? new Uint8Array(resp.body) : null;
			const response = new Response(body, {
				status: resp.status,
				headers,
			});

			pending.resolve(response);
			this.#actorPendingRequests.delete(requestIdStr);
		}
	}

	#handleResponseChunk(
		requestId: ArrayBuffer,
		chunk: tunnel.ToClientResponseChunk,
	) {
		const requestIdStr = bufferToString(requestId);
		const pending = this.#actorPendingRequests.get(requestIdStr);
		if (pending?.streamController) {
			pending.streamController.enqueue(new Uint8Array(chunk.body));
			if (chunk.finish) {
				pending.streamController.close();
				this.#actorPendingRequests.delete(requestIdStr);
			}
		}
	}

	#handleResponseAbort(requestId: ArrayBuffer) {
		const requestIdStr = bufferToString(requestId);
		const pending = this.#actorPendingRequests.get(requestIdStr);
		if (pending?.streamController) {
			pending.streamController.error(new Error("Response aborted"));
		}
		this.#actorPendingRequests.delete(requestIdStr);
	}

	#handleWebSocketOpenResponse(
		requestId: ArrayBuffer,
		open: tunnel.ToClientWebSocketOpen,
	) {
		const webSocketId = bufferToString(requestId);
		const adapter = this.#actorWebSockets.get(webSocketId);
		if (adapter) {
			adapter._handleOpen();
		}
	}

	#handleWebSocketMessageResponse(
		requestId: ArrayBuffer,
		msg: tunnel.ToClientWebSocketMessage,
	) {
		const webSocketId = bufferToString(requestId);
		const adapter = this.#actorWebSockets.get(webSocketId);
		if (adapter) {
			const data = msg.binary
				? new Uint8Array(msg.data)
				: new TextDecoder().decode(new Uint8Array(msg.data));

			adapter._handleMessage(data, msg.binary);
		}
	}

	#handleWebSocketCloseResponse(
		requestId: ArrayBuffer,
		close: tunnel.ToClientWebSocketClose,
	) {
		const webSocketId = bufferToString(requestId);
		const adapter = this.#actorWebSockets.get(webSocketId);
		if (adapter) {
			adapter._handleClose(
				close.code || undefined,
				close.reason || undefined,
			);
			this.#actorWebSockets.delete(webSocketId);
		}
	}
}

/** Converts a buffer to a string. Used for storing strings in a lookup map. */
function bufferToString(buffer: ArrayBuffer): string {
	return Buffer.from(buffer).toString("base64");
}

/** Generates a UUID as bytes. */
function generateUuidBuffer(): ArrayBuffer {
	const buffer = new Uint8Array(16);
	uuidv4(undefined, buffer);
	return buffer.buffer;
}
