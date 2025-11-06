import type * as protocol from "@rivetkit/engine-runner-protocol";
import type { MessageId, RequestId } from "@rivetkit/engine-runner-protocol";
import type { Logger } from "pino";
import { stringify as uuidstringify, v4 as uuidv4 } from "uuid";
import { logger } from "./log";
import type { ActorInstance, Runner } from "./mod";
import {
	stringifyToClientTunnelMessageKind,
	stringifyToServerTunnelMessageKind,
} from "./stringify";
import { unreachable } from "./utils";
import { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter";

const GC_INTERVAL = 60000; // 60 seconds
const MESSAGE_ACK_TIMEOUT = 5000; // 5 seconds
const WEBSOCKET_STATE_PERSIST_TIMEOUT = 30000; // 30 seconds

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

class RunnerShutdownError extends Error {
	constructor() {
		super("Runner shut down");
	}
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

	get log(): Logger | undefined {
		return this.#runner.log;
	}

	constructor(runner: Runner) {
		this.#runner = runner;
	}

	start(): void {
		this.#startGarbageCollector();
	}

	shutdown() {
		// NOTE: Pegboard WS already closed at this point, cannot send
		// anything. All teardown logic is handled by pegboard-runner.

		if (this.#gcInterval) {
			clearInterval(this.#gcInterval);
			this.#gcInterval = undefined;
		}

		// Reject all pending requests
		//
		// RunnerShutdownError will be explicitly ignored
		for (const [_, request] of this.#actorPendingRequests) {
			request.reject(new RunnerShutdownError());
		}
		this.#actorPendingRequests.clear();

		// Close all WebSockets
		//
		// The WebSocket close event with retry is automatically sent when the
		// runner WS closes, so we only need to notify the client that the WS
		// closed:
		// https://github.com/rivet-dev/rivet/blob/00d4f6a22da178a6f8115e5db50d96c6f8387c2e/engine/packages/pegboard-runner/src/lib.rs#L157
		for (const [_, ws] of this.#actorWebSockets) {
			// Only close non-hibernatable websockets to prevent sending
			// unnecessary close messages for websockets that will be hibernated
			if (!ws.canHibernate) {
				ws.__closeWithoutCallback(1000, "ws.tunnel_shutdown");
			}
		}
		this.#actorWebSockets.clear();
	}

	#sendMessage(
		requestId: RequestId,
		messageKind: protocol.ToServerTunnelMessageKind,
	) {
		// TODO: Switch this with runner WS
		if (!this.#runner.__webSocketReady()) {
			this.log?.warn({
				msg: "cannot send tunnel message, socket not connected to engine",
				requestId: idToStr(requestId),
				message: stringifyToServerTunnelMessageKind(messageKind),
			});
			return;
		}

		// Build message
		const messageId = generateUuidBuffer();

		const requestIdStr = idToStr(requestId);
		const messageIdStr = idToStr(messageId);
		this.#pendingTunnelMessages.set(messageIdStr, {
			sentAt: Date.now(),
			requestIdStr,
		});

		this.log?.debug({
			msg: "send tunnel msg",
			requestId: requestIdStr,
			messageId: messageIdStr,
			message: stringifyToServerTunnelMessageKind(messageKind),
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

		this.log?.debug({
			msg: "ack tunnel msg",
			requestId: idToStr(requestId),
			messageId: idToStr(messageId),
		});

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
					webSocket.__closeWithRetry(
						1000,
						"Message acknowledgment timeout",
					);

					// Clean up from actorWebSockets map
					this.#actorWebSockets.delete(requestIdStr);
				}
			}
		}

		// Remove timed out messages
		if (messagesToDelete.length > 0) {
			this.log?.warn({
				msg: "purging unacked tunnel messages, this indicates that the Rivet Engine is disconnected or not responding",
				count: messagesToDelete.length,
			});
			for (const messageId of messagesToDelete) {
				this.#pendingTunnelMessages.delete(messageId);
			}
		}
	}

	closeActiveRequests(actor: ActorInstance) {
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

		// Flush acks and close all WebSockets for this actor
		for (const requestIdStr of actor.webSockets) {
			const ws = this.#actorWebSockets.get(requestIdStr);
			if (ws) {
				ws.__closeWithRetry(1000, "Actor stopped");
				this.#actorWebSockets.delete(requestIdStr);
			}
		}
		actor.webSockets.clear();
	}

	async #fetch(
		actorId: string,
		requestId: protocol.RequestId,
		request: Request,
	): Promise<Response> {
		// Validate actor exists
		if (!this.#runner.hasActor(actorId)) {
			this.log?.warn({
				msg: "ignoring request for unknown actor",
				actorId,
			});

			// NOTE: This is a special response that will cause Guard to retry the request
			//
			// See should_retry_request_inner
			// https://github.com/rivet-dev/rivet/blob/222dae87e3efccaffa2b503de40ecf8afd4e31eb/engine/packages/guard-core/src/proxy_service.rs#L2458
			return new Response("Actor not found", {
				status: 503,
				headers: { "x-rivet-error": "runner.actor_not_found" },
			});
		}

		const fetchHandler = this.#runner.config.fetch(
			this.#runner,
			actorId,
			requestId,
			request,
		);

		if (!fetchHandler) {
			return new Response("Not Implemented", { status: 501 });
		}

		return fetchHandler;
	}

	async handleTunnelMessage(message: protocol.ToClientTunnelMessage) {
		const requestIdStr = idToStr(message.requestId);
		const messageIdStr = idToStr(message.messageId);
		this.log?.debug({
			msg: "receive tunnel msg",
			requestId: requestIdStr,
			messageId: messageIdStr,
			message: stringifyToClientTunnelMessageKind(message.messageKind),
		});

		if (message.messageKind.tag === "TunnelAck") {
			// Mark pending message as acknowledged and remove it
			const pending = this.#pendingTunnelMessages.get(messageIdStr);
			if (pending) {
				const didDelete =
					this.#pendingTunnelMessages.delete(messageIdStr);
				if (!didDelete) {
					this.log?.warn({
						msg: "received tunnel ack for nonexistent message",
						requestId: requestIdStr,
						messageId: messageIdStr,
					});
				}
			}
		} else {
			switch (message.messageKind.tag) {
				case "ToClientRequestStart":
					this.#sendAck(message.requestId, message.messageId);

					await this.#handleRequestStart(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientRequestChunk":
					this.#sendAck(message.requestId, message.messageId);

					await this.#handleRequestChunk(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientRequestAbort":
					this.#sendAck(message.requestId, message.messageId);

					await this.#handleRequestAbort(message.requestId);
					break;
				case "ToClientWebSocketOpen":
					this.#sendAck(message.requestId, message.messageId);

					await this.#handleWebSocketOpen(
						message.requestId,
						message.messageKind.val,
					);
					break;
				case "ToClientWebSocketMessage": {
					this.#sendAck(message.requestId, message.messageId);

					const _unhandled = await this.#handleWebSocketMessage(
						message.requestId,
						message.messageKind.val,
					);
					break;
				}
				case "ToClientWebSocketClose":
					this.#sendAck(message.requestId, message.messageId);

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
		// Track this request for the actor
		const requestIdStr = idToStr(requestId);
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
					requestId,
					streamingRequest,
				);
				await this.#sendResponse(requestId, response);
			} else {
				// Non-streaming request
				const response = await this.#fetch(
					req.actorId,
					requestId,
					request,
				);
				await this.#sendResponse(requestId, response);
			}
		} catch (error) {
			if (error instanceof RunnerShutdownError) {
				this.log?.debug({ msg: "catught runner shutdown error" });
			} else {
				this.log?.error({ msg: "error handling request", error });
				this.#sendResponseError(
					requestId,
					500,
					"Internal Server Error",
				);
			}
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
		chunk: protocol.ToClientRequestChunk,
	) {
		const requestIdStr = idToStr(requestId);
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
		const requestIdStr = idToStr(requestId);
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
		requestId: protocol.RequestId,
		open: protocol.ToClientWebSocketOpen,
	) {
		const requestIdStr = idToStr(requestId);

		// Validate actor exists
		const actor = this.#runner.getActor(open.actorId);
		if (!actor) {
			this.log?.warn({
				msg: "ignoring websocket for unknown actor",
				actorId: open.actorId,
			});

			// NOTE: Closing a WebSocket before open is equivalent to a Service
			// Unavailable error and will cause Guard to retry the request
			//
			// See
			// https://github.com/rivet-dev/rivet/blob/222dae87e3efccaffa2b503de40ecf8afd4e31eb/engine/packages/pegboard-gateway/src/lib.rs#L238
			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketClose",
				val: {
					code: 1011,
					reason: "Actor not found",
					retry: false,
				},
			});
			return;
		}

		const websocketHandler = this.#runner.config.websocket;

		if (!websocketHandler) {
			this.log?.error({
				msg: "no websocket handler configured for tunnel",
			});
			// Send close immediately
			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketClose",
				val: {
					code: 1011,
					reason: "Not Implemented",
					retry: false,
				},
			});
			return;
		}

		// Close existing WebSocket if one already exists for this request ID.
		// There should always be a close message sent before another open
		// message for the same message ID.
		//
		// This should never occur if all is functioning correctly, but this
		// prevents any edge case that would result in duplicate WebSockets for
		// the same request.
		const existingAdapter = this.#actorWebSockets.get(requestIdStr);
		if (existingAdapter) {
			this.log?.warn({
				msg: "closing existing websocket for duplicate open event for the same request id",
				requestId: requestIdStr,
			});
			// Close without sending a message through the tunnel since the server
			// already knows about the new connection
			existingAdapter.__closeWithoutCallback(1000, "ws.duplicate_open");
		}

		// Track this WebSocket for the actor
		if (actor) {
			actor.webSockets.add(requestIdStr);
		}

		try {
			// Create WebSocket adapter
			const adapter = new WebSocketTunnelAdapter(
				requestIdStr,
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
				(code?: number, reason?: string, retry: boolean = false) => {
					// Send close through tunnel
					this.#sendMessage(requestId, {
						tag: "ToServerWebSocketClose",
						val: {
							code: code || null,
							reason: reason || null,
							retry,
						},
					});

					// Remove from map
					this.#actorWebSockets.delete(requestIdStr);

					// Clean up actor tracking
					if (actor) {
						actor.webSockets.delete(requestIdStr);
					}
				},
			);

			// Store adapter
			this.#actorWebSockets.set(requestIdStr, adapter);

			// Convert headers to map
			//
			// We need to manually ensure the original Upgrade/Connection WS
			// headers are present
			const headerInit: Record<string, string> = {};
			if (open.headers) {
				for (const [k, v] of open.headers as ReadonlyMap<
					string,
					string
				>) {
					headerInit[k] = v;
				}
			}
			headerInit["Upgrade"] = "websocket";
			headerInit["Connection"] = "Upgrade";

			const request = new Request(`http://localhost${open.path}`, {
				method: "GET",
				headers: headerInit,
			});

			// Send open confirmation
			const hibernationConfig =
				this.#runner.config.getActorHibernationConfig(
					actor.actorId,
					requestId,
					request,
				);
			adapter.canHibernate = hibernationConfig.enabled;

			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketOpen",
				val: {
					canHibernate: hibernationConfig.enabled,
					lastMsgIndex: BigInt(hibernationConfig.lastMsgIndex ?? -1),
				},
			});

			// Notify adapter that connection is open
			adapter._handleOpen(requestId);

			// Call websocket handler
			await websocketHandler(
				this.#runner,
				open.actorId,
				adapter,
				requestId,
				request,
			);
		} catch (error) {
			this.log?.error({ msg: "error handling websocket open", error });
			// Send close on error
			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketClose",
				val: {
					code: 1011,
					reason: "Server Error",
					retry: false,
				},
			});

			this.#actorWebSockets.delete(requestIdStr);

			// Clean up actor tracking
			if (actor) {
				actor.webSockets.delete(requestIdStr);
			}
		}
	}

	/// Returns false if the message was sent off
	async #handleWebSocketMessage(
		requestId: ArrayBuffer,
		msg: protocol.ToClientWebSocketMessage,
	): Promise<boolean> {
		const requestIdStr = idToStr(requestId);
		const adapter = this.#actorWebSockets.get(requestIdStr);
		if (adapter) {
			const data = msg.binary
				? new Uint8Array(msg.data)
				: new TextDecoder().decode(new Uint8Array(msg.data));

			return adapter._handleMessage(
				requestId,
				data,
				msg.index,
				msg.binary,
			);
		} else {
			return true;
		}
	}

	__ackWebsocketMessage(requestId: ArrayBuffer, index: number) {
		this.log?.debug({
			msg: "ack ws msg",
			requestId: idToStr(requestId),
			index,
		});

		if (index < 0 || index > 65535)
			throw new Error("invalid websocket ack index");

		// Send the ack message
		this.#sendMessage(requestId, {
			tag: "ToServerWebSocketMessageAck",
			val: {
				index,
			},
		});
	}

	async #handleWebSocketClose(
		requestId: ArrayBuffer,
		close: protocol.ToClientWebSocketClose,
	) {
		const requestIdStr = idToStr(requestId);
		const adapter = this.#actorWebSockets.get(requestIdStr);
		if (adapter) {
			adapter._handleClose(
				requestId,
				close.code || undefined,
				close.reason || undefined,
			);
			this.#actorWebSockets.delete(requestIdStr);
		}
	}
}

/** Generates a UUID as bytes. */
function generateUuidBuffer(): ArrayBuffer {
	const buffer = new Uint8Array(16);
	uuidv4(undefined, buffer);
	return buffer.buffer;
}

function idToStr(id: ArrayBuffer): string {
	return uuidstringify(new Uint8Array(id));
}
