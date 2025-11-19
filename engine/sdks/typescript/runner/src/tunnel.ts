import type * as protocol from "@rivetkit/engine-runner-protocol";
import type { MessageId, RequestId } from "@rivetkit/engine-runner-protocol";
import type { Logger } from "pino";
import {
	parse as uuidparse,
	stringify as uuidstringify,
	v4 as uuidv4,
} from "uuid";
import type { Runner, RunnerActor } from "./mod";
import {
	stringifyToClientTunnelMessageKind,
	stringifyToServerTunnelMessageKind,
} from "./stringify";
import { unreachable } from "./utils";
import {
	HIBERNATABLE_SYMBOL,
	WebSocketTunnelAdapter,
} from "./websocket-tunnel-adapter";

const GC_INTERVAL = 60000; // 60 seconds
const MESSAGE_ACK_TIMEOUT = 5000; // 5 seconds

export interface PendingRequest {
	resolve: (response: Response) => void;
	reject: (error: Error) => void;
	streamController?: ReadableStreamDefaultController<Uint8Array>;
	actorId?: string;
}

export interface HibernatingWebSocketMetadata {
	requestId: RequestId;
	path: string;
	headers: Record<string, string>;
	messageIndex: number;
}

export interface PendingTunnelMessage {
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

	/** Maps request IDs to actor IDs for lookup */
	#requestToActor: Map<string, string> = new Map();

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

		// Reject all pending requests and close all WebSockets for all actors
		// RunnerShutdownError will be explicitly ignored
		for (const [_actorId, actor] of this.#runner.actors) {
			// Reject all pending requests for this actor
			for (const [_, request] of actor.pendingRequests) {
				request.reject(new RunnerShutdownError());
			}
			actor.pendingRequests.clear();

			// Close all WebSockets for this actor
			// The WebSocket close event with retry is automatically sent when the
			// runner WS closes, so we only need to notify the client that the WS
			// closed:
			// https://github.com/rivet-dev/rivet/blob/00d4f6a22da178a6f8115e5db50d96c6f8387c2e/engine/packages/pegboard-runner/src/lib.rs#L157
			for (const [_, ws] of actor.webSockets) {
				// Only close non-hibernatable websockets to prevent sending
				// unnecessary close messages for websockets that will be hibernated
				if (!ws[HIBERNATABLE_SYMBOL]) {
					ws._closeWithoutCallback(1000, "ws.tunnel_shutdown");
				}
			}
			actor.webSockets.clear();
		}

		// Clear the request-to-actor mapping
		this.#requestToActor.clear();
	}

	async restoreHibernatingRequests(
		actorId: string,
		requestIds: readonly RequestId[],
	) {
		this.log?.debug({
			msg: "restoring hibernating requests",
			actorId,
			requests: requestIds.length,
		});

		// Load all persisted metadata
		const metaEntries =
			await this.#runner.config.hibernatableWebSocket.loadAll(actorId);

		// Create maps for efficient lookup
		const requestIdMap = new Map<string, RequestId>();
		for (const requestId of requestIds) {
			requestIdMap.set(idToStr(requestId), requestId);
		}

		const metaMap = new Map<string, HibernatingWebSocketMetadata>();
		for (const meta of metaEntries) {
			metaMap.set(idToStr(meta.requestId), meta);
		}

		// Track all background operations
		const backgroundOperations: Promise<void>[] = [];

		// Process connected WebSockets
		let connectedButNotLoadedCount = 0;
		let restoredCount = 0;
		for (const [requestIdStr, requestId] of requestIdMap) {
			const meta = metaMap.get(requestIdStr);

			if (!meta) {
				// Connected but not loaded (not persisted) - close it
				//
				// This may happen if
				this.log?.warn({
					msg: "closing websocket that is not persisted",
					requestId: requestIdStr,
				});

				this.#sendMessage(requestId, {
					tag: "ToServerWebSocketClose",
					val: {
						code: 1000,
						reason: "ws.meta_not_found_during_restore",
						hibernate: false,
					},
				});

				connectedButNotLoadedCount++;
			} else {
				// Both connected and persisted - restore it
				const request = buildRequestForWebSocket(
					meta.path,
					meta.headers,
				);

				// This will call `runner.config.websocket` under the hood to
				// attach the event listeners to the WebSocket.
				// Track this operation to ensure it completes
				const restoreOperation = this.#createWebSocket(
					actorId,
					requestId,
					requestIdStr,
					true,
					true,
					meta.messageIndex,
					request,
					meta.path,
					meta.headers,
					false,
				)
					.then(() => {
						this.log?.info({
							msg: "connection successfully restored",
							actorId,
							requestId: requestIdStr,
						});
					})
					.catch((err) => {
						this.log?.error({
							msg: "error creating websocket during restore",
							requestId: requestIdStr,
							err,
						});

						// Close the WebSocket on error
						this.#sendMessage(requestId, {
							tag: "ToServerWebSocketClose",
							val: {
								code: 1011,
								reason: "ws.restore_error",
								hibernate: false,
							},
						});
					});

				backgroundOperations.push(restoreOperation);
				restoredCount++;
			}
		}

		// Process loaded but not connected (stale) - remove them
		let loadedButNotConnectedCount = 0;
		for (const [requestIdStr, meta] of metaMap) {
			if (!requestIdMap.has(requestIdStr)) {
				this.log?.warn({
					msg: "removing stale persisted websocket",
					requestId: requestIdStr,
				});

				const request = buildRequestForWebSocket(
					meta.path,
					meta.headers,
				);

				// Create adapter to register user's event listeners.
				// Pass engineAlreadyClosed=true so close callback won't send tunnel message.
				// Track this operation to ensure it completes
				const cleanupOperation = this.#createWebSocket(
					actorId,
					meta.requestId,
					requestIdStr,
					true,
					true,
					meta.messageIndex,
					request,
					meta.path,
					meta.headers,
					true,
				)
					.then((adapter) => {
						// Close the adapter normally - this will fire user's close event handler
						// (which should clean up persistence) and trigger the close callback
						// (which will clean up maps but skip sending tunnel message)
						adapter.close(1000, "ws.stale_metadata");
					})
					.catch((err) => {
						this.log?.error({
							msg: "error creating stale websocket during restore",
							requestId: requestIdStr,
							err,
						});
					});

				backgroundOperations.push(cleanupOperation);
				loadedButNotConnectedCount++;
			}
		}

		// Wait for all background operations to complete before finishing
		await Promise.allSettled(backgroundOperations);

		this.log?.info({
			msg: "restored hibernatable websockets",
			actorId,
			restoredCount,
			connectedButNotLoadedCount,
			loadedButNotConnectedCount,
		});
	}

	/**
	 * Called from WebSocketOpen message and when restoring hibernatable WebSockets.
	 *
	 * engineAlreadyClosed will be true if this is only being called to trigger
	 * the close callback and not to send a close message to the server. This
	 * is used specifically to clean up zombie WebSocket connections.
	 */
	async #createWebSocket(
		actorId: string,
		requestId: RequestId,
		requestIdStr: string,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
		messageIndex: number,
		request: Request,
		path: string,
		headers: Record<string, string>,
		engineAlreadyClosed: boolean,
	): Promise<WebSocketTunnelAdapter> {
		this.log?.debug({
			msg: "createWebSocket creating adapter",
			actorId,
			requestIdStr,
			isHibernatable,
			path,
		});
		// Create WebSocket adapter
		const adapter = new WebSocketTunnelAdapter(
			this,
			actorId,
			requestIdStr,
			isHibernatable,
			messageIndex,
			isRestoringHibernatable,
			request,
			(data: ArrayBuffer | string, isBinary: boolean) => {
				// Send message through tunnel
				const dataBuffer =
					typeof data === "string"
						? (new TextEncoder().encode(data).buffer as ArrayBuffer)
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
				// Send close through tunnel if engine doesn't already know it's closed
				if (!engineAlreadyClosed) {
					this.#sendMessage(requestId, {
						tag: "ToServerWebSocketClose",
						val: {
							code: code || null,
							reason: reason || null,
							hibernate: false,
						},
					});
				}

				// Clean up actor tracking
				const actor = this.#runner.getActor(actorId);
				if (actor) {
					actor.webSockets.delete(requestIdStr);
				}

				// Clean up request-to-actor mapping
				this.#requestToActor.delete(requestIdStr);
			},
		);

		// Get actor and add websocket to it
		const actor = this.#runner.getActor(actorId);
		if (!actor) {
			throw new Error(`Actor ${actorId} not found`);
		}

		actor.webSockets.set(requestIdStr, adapter);
		this.#requestToActor.set(requestIdStr, actorId);

		// Call WebSocket handler. This handler will add event listeners
		// for `open`, etc.
		await this.#runner.config.websocket(
			this.#runner,
			actorId,
			adapter,
			requestId,
			request,
			path,
			headers,
			isHibernatable,
			isRestoringHibernatable,
		);

		return adapter;
	}

	getRequestActor(requestIdStr: string): RunnerActor | undefined {
		const actorId = this.#requestToActor.get(requestIdStr);
		if (!actorId) {
			this.log?.warn({
				msg: "missing requestToActor entry",
				requestId: requestIdStr,
			});
			return undefined;
		}

		const actor = this.#runner.getActor(actorId);
		if (!actor) {
			this.log?.warn({
				msg: "missing actor for requestToActor lookup",
				requestId: requestIdStr,
				actorId,
			});
			return undefined;
		}

		return actor;
	}

	#sendMessage(
		requestId: RequestId,
		messageKind: protocol.ToServerTunnelMessageKind,
	) {
		// TODO: Switch this with runner WS
		if (!this.#runner.__webSocketReady()) {
			this.log?.warn({
				msg: "cannot send tunnel message, socket not connected to engine. tunnel data dropped.",
				requestId: idToStr(requestId),
				message: stringifyToServerTunnelMessageKind(messageKind),
			});
			return;
		}

		// Build message
		const messageId = generateUuidBuffer();

		const requestIdStr = idToStr(requestId);
		const messageIdStr = idToStr(messageId);

		// Store the pending message in the actor's map
		const actor = this.getRequestActor(requestIdStr);
		if (actor) {
			actor.pendingTunnelMessages.set(messageIdStr, {
				sentAt: Date.now(),
				requestIdStr,
			});
		}

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
		let totalMessagesToDelete = 0;

		// Iterate through all actors
		for (const [_actorId, actor] of this.#runner.actors) {
			const messagesToDelete: string[] = [];

			for (const [
				messageId,
				pendingMessage,
			] of actor.pendingTunnelMessages) {
				// Check if message is older than timeout
				if (now - pendingMessage.sentAt > MESSAGE_ACK_TIMEOUT) {
					messagesToDelete.push(messageId);

					const requestIdStr = pendingMessage.requestIdStr;

					// Check if this is an HTTP request
					const pendingRequest =
						actor.pendingRequests.get(requestIdStr);
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

						// Clean up from pendingRequests map
						actor.pendingRequests.delete(requestIdStr);
					}

					// Check if this is a WebSocket
					const webSocket = actor.webSockets.get(requestIdStr);
					if (webSocket) {
						// Close the WebSocket connection
						webSocket.close(1000, "ws.ack_timeout");

						// Clean up from webSockets map
						actor.webSockets.delete(requestIdStr);
					}

					// Clean up request-to-actor mapping
					this.#requestToActor.delete(requestIdStr);
				}
			}

			// Remove timed out messages for this actor
			for (const messageId of messagesToDelete) {
				actor.pendingTunnelMessages.delete(messageId);
			}

			totalMessagesToDelete += messagesToDelete.length;
		}

		// Log if we purged any messages
		if (totalMessagesToDelete > 0) {
			this.log?.warn({
				msg: "purging unacked tunnel messages, this indicates that the Rivet Engine is disconnected or not responding",
				count: totalMessagesToDelete,
			});
		}
	}

	closeActiveRequests(actor: RunnerActor) {
		const actorId = actor.actorId;

		// Terminate all requests for this actor. This will no send a
		// ToServerResponse* message since the actor will no longer be loaded.
		// The gateway is responsible for closing the request.
		for (const [requestIdStr, pending] of actor.pendingRequests) {
			pending.reject(new Error(`Actor ${actorId} stopped`));
			this.#requestToActor.delete(requestIdStr);
		}

		// Close all WebSockets. Only send close event to non-HWS. The gateway is
		// responsible for hibernating HWS and closing regular WS.
		for (const [requestIdStr, ws] of actor.webSockets) {
			const isHibernatable = ws[HIBERNATABLE_SYMBOL];
			if (!isHibernatable) {
				ws._closeWithoutCallback(1000, "actor.stopped");
			}
			this.#requestToActor.delete(requestIdStr);
		}
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
			const actor = this.getRequestActor(requestIdStr);
			if (actor) {
				const didDelete =
					actor.pendingTunnelMessages.delete(messageIdStr);
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

					this.#handleWebSocketMessage(
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
		if (!actor) {
			this.log?.warn({
				msg: "actor does not exist in handleRequestStart, request will leak",
				actorId: req.actorId,
				requestId: requestIdStr,
			});
			return;
		}

		// Add to request-to-actor mapping
		this.#requestToActor.set(requestIdStr, req.actorId);

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
							actor.pendingRequests.get(requestIdStr);
						if (existing) {
							existing.streamController = controller;
							existing.actorId = req.actorId;
						} else {
							actor.pendingRequests.set(requestIdStr, {
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
				await this.#sendResponse(
					actor.actorId,
					actor.generation,
					requestId,
					response,
				);
			} else {
				// Non-streaming request
				const response = await this.#fetch(
					req.actorId,
					requestId,
					request,
				);
				await this.#sendResponse(
					actor.actorId,
					actor.generation,
					requestId,
					response,
				);
			}
		} catch (error) {
			if (error instanceof RunnerShutdownError) {
				this.log?.debug({ msg: "catught runner shutdown error" });
			} else {
				this.log?.error({ msg: "error handling request", error });
				this.#sendResponseError(
					actor.actorId,
					actor.generation,
					requestId,
					500,
					"Internal Server Error",
				);
			}
		} finally {
			// Clean up request tracking
			if (this.#runner.hasActor(req.actorId, actor.generation)) {
				actor.pendingRequests.delete(requestIdStr);
				this.#requestToActor.delete(requestIdStr);
			}
		}
	}

	async #handleRequestChunk(
		requestId: ArrayBuffer,
		chunk: protocol.ToClientRequestChunk,
	) {
		const requestIdStr = idToStr(requestId);
		const actor = this.getRequestActor(requestIdStr);
		if (actor) {
			const pending = actor.pendingRequests.get(requestIdStr);
			if (pending?.streamController) {
				pending.streamController.enqueue(new Uint8Array(chunk.body));
				if (chunk.finish) {
					pending.streamController.close();
					actor.pendingRequests.delete(requestIdStr);
					this.#requestToActor.delete(requestIdStr);
				}
			}
		}
	}

	async #handleRequestAbort(requestId: ArrayBuffer) {
		const requestIdStr = idToStr(requestId);
		const actor = this.getRequestActor(requestIdStr);
		if (actor) {
			const pending = actor.pendingRequests.get(requestIdStr);
			if (pending?.streamController) {
				pending.streamController.error(new Error("Request aborted"));
			}
			actor.pendingRequests.delete(requestIdStr);
			this.#requestToActor.delete(requestIdStr);
		}
	}

	async #sendResponse(
		actorId: string,
		generation: number,
		requestId: ArrayBuffer,
		response: Response,
	) {
		if (this.#runner.hasActor(actorId, generation)) {
			this.log?.warn({
				msg: "actor not loaded to send response, assuming gateway has closed request",
				actorId,
				generation,
				requestId,
			});
			return;
		}

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

		// Send as non-streaming response if actor has not stopped
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
		actorId: string,
		generation: number,
		requestId: ArrayBuffer,
		status: number,
		message: string,
	) {
		if (this.#runner.hasActor(actorId, generation)) {
			this.log?.warn({
				msg: "actor not loaded to send response, assuming gateway has closed request",
				actorId,
				generation,
				requestId,
			});
			return;
		}

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
		// NOTE: This method is safe to be async since we will not receive any
		// further WebSocket events until we send a ToServerWebSocketOpen
		// tunnel message. We can do any async logic we need to between thoes two events.
		//
		// Sedning a ToServerWebSocketClose will terminate the WebSocket early.

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
					hibernate: false,
				},
			});
			return;
		}

		// Close existing WebSocket if one already exists for this request ID.
		// This should never happen, but prevents any potential duplicate
		// WebSockets from retransmits.
		const existingAdapter = actor.webSockets.get(requestIdStr);
		if (existingAdapter) {
			this.log?.warn({
				msg: "closing existing websocket for duplicate open event for the same request id",
				requestId: requestIdStr,
			});
			// Close without sending a message through the tunnel since the server
			// already knows about the new connection
			existingAdapter._closeWithoutCallback(1000, "ws.duplicate_open");
		}

		// Create WebSocket
		try {
			const request = buildRequestForWebSocket(
				open.path,
				Object.fromEntries(open.headers),
			);

			const canHibernate =
				this.#runner.config.hibernatableWebSocket.canHibernate(
					actor.actorId,
					requestId,
					request,
				);

			// #createWebSocket will call `runner.config.websocket` under the
			// hood to add the event listeners for open, etc. If this handler
			// throws, then the WebSocket will be closed before sending the
			// open event.
			const adapter = await this.#createWebSocket(
				actor.actorId,
				requestId,
				requestIdStr,
				canHibernate,
				false,
				0,
				request,
				open.path,
				Object.fromEntries(open.headers),
				false,
			);

			// Open the WebSocket after `config.socket` so (a) the event
			// handlers can be added and (b) any errors in `config.websocket`
			// will cause the WebSocket to terminate before the open event.
			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketOpen",
				val: {
					canHibernate,
				},
			});

			// Dispatch open event
			adapter._handleOpen(requestId);
		} catch (error) {
			this.log?.error({ msg: "error handling websocket open", error });

			// TODO: Call close event on adapter if needed

			// Send close on error
			this.#sendMessage(requestId, {
				tag: "ToServerWebSocketClose",
				val: {
					code: 1011,
					reason: "Server Error",
					hibernate: false,
				},
			});

			// Clean up actor tracking
			actor.webSockets.delete(requestIdStr);
			this.#requestToActor.delete(requestIdStr);
		}
	}

	#handleWebSocketMessage(
		requestId: ArrayBuffer,
		msg: protocol.ToClientWebSocketMessage,
	) {
		// NOTE: This method cannot be async in order to ensure in-order
		// message processing.

		const requestIdStr = idToStr(requestId);
		const actor = this.getRequestActor(requestIdStr);
		if (actor) {
			const adapter = actor.webSockets.get(requestIdStr);
			if (adapter) {
				const data = msg.binary
					? new Uint8Array(msg.data)
					: new TextDecoder().decode(new Uint8Array(msg.data));

				adapter._handleMessage(requestId, data, msg.index, msg.binary);
				return;
			}
		}

		// TODO: This will never retransmit the socket and the socket will close
		this.log?.warn({
			msg: "missing websocket for incoming websocket message, this may indicate the actor stopped before processing a message",
			requestId,
		});
	}

	sendHibernatableWebSocketMessageAck(requestId: ArrayBuffer, index: number) {
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
		const actor = this.getRequestActor(requestIdStr);
		if (actor) {
			const adapter = actor.webSockets.get(requestIdStr);
			if (adapter) {
				// We don't need to send a close response
				adapter._handleClose(
					requestId,
					close.code || undefined,
					close.reason || undefined,
				);
				actor.webSockets.delete(requestIdStr);
				this.#requestToActor.delete(requestIdStr);
			}
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

/**
 * Builds a request that represents the incoming request for a given WebSocket.
 *
 * This request is not a real request and will never be sent. It's used to be passed to the actor to behave like a real incoming request.
 */
function buildRequestForWebSocket(
	path: string,
	headers: Record<string, string>,
): Request {
	// We need to manually ensure the original Upgrade/Connection WS
	// headers are present
	const fullHeaders = {
		...headers,
		Upgrade: "websocket",
		Connection: "Upgrade",
	};

	if (!path.startsWith("/")) {
		throw new Error("path must start with leading slash");
	}

	const request = new Request(`http://actor${path}`, {
		method: "GET",
		headers: fullHeaders,
	});

	return request;
}
