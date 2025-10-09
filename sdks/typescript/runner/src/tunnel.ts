import * as protocol from "@rivetkit/engine-runner-protocol";
import type { RequestId, MessageId } from "@rivetkit/engine-runner-protocol";
import { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter";
import type { Runner, ActorInstance } from "./mod";
import { v4 as uuidv4 } from "uuid";
import { logger } from "./log";
import { unreachable } from "./utils";

const GC_INTERVAL = 60000; // 60 seconds
const MESSAGE_ACK_TIMEOUT = 5000; // 5 seconds

interface PendingRequest {
	resolve: (response: Response) => void;
	reject: (error: Error) => void;
	streamController?: ReadableStreamDefaultController<Uint8Array>;
	actorId?: string;
}

interface PendingTunnelMessage {
	sentAt: number;
	requestIdStr: string;
}

export class Tunnel {
	#runner: Runner;

	/** Requests over the tunnel to the actor that are in flight. */
	#actorPendingRequests: Map<string, PendingRequest> = new Map();
	/** WebSockets over the tunnel to the actor that are in flight. */
	#actorWebSockets: Map<string, WebSocketTunnelAdapter> = new Map();

	/** Messages sent from the actor over the tunnel that have not been acked by the gateway. */
	#pendingTunnelMessages: Map<string, PendingTunnelMessage> = new Map();

	#gcInterval?: NodeJS.Timeout;

	constructor(runner: Runner) {
		this.#runner = runner;
	}

	start(): void {
		this.#startGarbageCollector();
	}

	shutdown() {
		if (this.#gcInterval) {
			clearInterval(this.#gcInterval);
			this.#gcInterval = undefined;
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

	#sendMessage(
		requestId: RequestId,
		messageKind: protocol.ToServerTunnelMessageKind,
	) {
		// TODO: Switch this with runner WS
		if (!this.#runner.__webSocketReady()) {
			logger()?.warn(
				"cannot send tunnel message, socket not connected to engine",
			);
			return;
		}

		// Build message
		const messageId = generateUuidBuffer();

		const requestIdStr = bufferToString(requestId);
		this.#pendingTunnelMessages.set(bufferToString(messageId), {
			sentAt: Date.now(),
			requestIdStr,
		});

		// Send message
		const message: protocol.ToServer = {
			tag: "ToServerTunnelMessage",
			val: {
				requestId,
				messageId,
				messageKind,
			},
		};
		this.#runner.__sendToServer(message);
	}

	#sendAck(requestId: RequestId, messageId: MessageId) {
		if (!this.#runner.__webSocketReady()) {
			return;
		}

		const message: protocol.ToServer = {
			tag: "ToServerTunnelMessage",
			val: {
				requestId,
				messageId,
				messageKind: { tag: "TunnelAck", val: null },
			},
		};

		this.#runner.__sendToServer(message);
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

		for (const [messageId, pendingMessage] of this.#pendingTunnelMessages) {
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
		if (messagesToDelete.length > 0) {
			logger()?.warn({
				msg: "purging unacked tunnel messages, this indicates that the Rivet Engine is disconnected or not responding",
				count: messagesToDelete.length,
			});
			for (const messageId of messagesToDelete) {
				this.#pendingTunnelMessages.delete(messageId);
			}
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

	async handleTunnelMessage(message: protocol.ToClientTunnelMessage) {
		if (message.messageKind.tag === "TunnelAck") {
			// Mark pending message as acknowledged and remove it
			const msgIdStr = bufferToString(message.messageId);
			const pending = this.#pendingTunnelMessages.get(msgIdStr);
			if (pending) {
				this.#pendingTunnelMessages.delete(msgIdStr);
			}
		} else {
			this.#sendAck(message.requestId, message.messageId);
			switch (message.messageKind.tag) {
				case "ToClientRequestStart":
					await this.#handleRequestStart(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientRequestChunk":
					await this.#handleRequestChunk(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientRequestAbort":
					await this.#handleRequestAbort(message.requestId);
					break;
				case "ToClientWebSocketOpen":
					await this.#handleWebSocketOpen(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientWebSocketMessage":
					await this.#handleWebSocketMessage(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientWebSocketClose":
					await this.#handleWebSocketClose(
						message.requestId,
						message.messageKind.val,
					);
					break;
				default:
					unreachable(message.messageKind);
			}
		}
	}

	async #handleRequestStart(
		requestId: ArrayBuffer,
		req: protocol.ToClientRequestStart,
	) {
		// Load actor
		const requestIdStr = bufferToString(requestId);
		const actor = this.#runner.getActor(req.actorId);
		if (!actor) {
			logger()?.warn({
				msg: "ignoring request for unknown actor",
				actorId: req.actorId,
			});
			this.#sendResponseError(requestId, 404, "Actor not found");
			return;
		}

		try {
			// Track request
			actor.requests.add(requestIdStr);

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
			// Always try to delete from actorPendingRequests in case it was added
			this.#actorPendingRequests.delete(requestIdStr);

			// Clean up actor tracking (actor may have been removed in the meantime)
			const currentActor = this.#runner.getActor(req.actorId);
			if (currentActor) {
				currentActor.requests.delete(requestIdStr);
			}
		}
	}

	async #handleRequestChunk(
		requestId: ArrayBuffer,
		chunk: protocol.ToClientRequestChunk,
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
			tag: "ToServerResponseStart",
			val: {
				status: response.status as protocol.u16,
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
			tag: "ToServerResponseStart",
			val: {
				status: status as protocol.u16,
				headers,
				body: new TextEncoder().encode(message).buffer as ArrayBuffer,
				stream: false,
			},
		});
	}

	async #handleWebSocketOpen(
		requestId: ArrayBuffer,
		open: protocol.ToClientWebSocketOpen,
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
				tag: "ToServerWebSocketClose",
				val: {
					code: 1011,
					reason: "Actor not found",
				},
			});
			return;
		}

		const websocketHandler = this.#runner.config.websocket;

		if (!websocketHandler) {
			logger()?.error({
				msg: "no websocket handler configured for tunnel",
			});
			// Send close immediately
			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketClose",
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
						tag: "ToServerWebSocketMessage",
						val: {
							data: dataBuffer,
							binary: isBinary,
						},
					});
				},
				(code?: number, reason?: string) => {
					// Send close through tunnel
					this.#sendMessage(requestId, {
						tag: "ToServerWebSocketClose",
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
				tag: "ToServerWebSocketOpen",
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
				tag: "ToServerWebSocketClose",
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
		msg: protocol.ToServerWebSocketMessage,
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
		close: protocol.ToServerWebSocketClose,
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
