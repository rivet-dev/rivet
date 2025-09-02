import WebSocket from "ws";
import * as tunnel from "@rivetkit/engine-tunnel-protocol";
import { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter.js";
import { calculateBackoff } from "./utils.js";

export class Tunnel {
	#pegboardTunnelUrl: string;
	#ws?: WebSocket;
	#pendingRequests: Map<bigint, {
		resolve: (response: Response) => void;
		reject: (error: Error) => void;
		streamController?: ReadableStreamDefaultController<Uint8Array>;
		actorId?: string;
	}> = new Map();
	#webSockets: Map<bigint, WebSocketTunnelAdapter> = new Map();
	#shutdown = false;
	#reconnectTimeout?: NodeJS.Timeout;
	#reconnectAttempt = 0;
	
	// Track actors and their connections
	#activeActors: Set<string> = new Set();
	#actorRequests: Map<string, Set<bigint>> = new Map();
	#actorWebSockets: Map<string, Set<bigint>> = new Map();
	
	// Callbacks
	#onConnected?: () => void;
	#onDisconnected?: () => void;
	#fetchHandler?: (actorId: string, request: Request) => Promise<Response>;
	#websocketHandler?: (actorId: string, ws: any, request: Request) => Promise<void>;

	constructor(pegboardTunnelUrl: string) {
		this.#pegboardTunnelUrl = pegboardTunnelUrl;
	}

	setCallbacks(options: {
		onConnected?: () => void;
		onDisconnected?: () => void;
		fetch?: (actorId: string, request: Request) => Promise<Response>;
		websocket?: (actorId: string, ws: any, request: Request) => Promise<void>;
	}) {
		this.#onConnected = options.onConnected;
		this.#onDisconnected = options.onDisconnected;
		this.#fetchHandler = options.fetch;
		this.#websocketHandler = options.websocket;
	}

	start(): void {
		if (this.#ws?.readyState === WebSocket.OPEN) {
			return;
		}
		
		this.#connect();
	}

	shutdown() {
		this.#shutdown = true;
		
		if (this.#reconnectTimeout) {
			clearTimeout(this.#reconnectTimeout);
			this.#reconnectTimeout = undefined;
		}

		if (this.#ws) {
			this.#ws.close();
			this.#ws = undefined;
		}

		// Reject all pending requests
		for (const [_, request] of this.#pendingRequests) {
			request.reject(new Error("Tunnel shutting down"));
		}
		this.#pendingRequests.clear();

		// Close all WebSockets
		for (const [_, ws] of this.#webSockets) {
			ws.close();
		}
		this.#webSockets.clear();
		
		// Clear actor tracking
		this.#activeActors.clear();
		this.#actorRequests.clear();
		this.#actorWebSockets.clear();
	}

	registerActor(actorId: string) {
		this.#activeActors.add(actorId);
		this.#actorRequests.set(actorId, new Set());
		this.#actorWebSockets.set(actorId, new Set());
	}

	unregisterActor(actorId: string) {
		this.#activeActors.delete(actorId);
		
		// Terminate all requests for this actor
		const requests = this.#actorRequests.get(actorId);
		if (requests) {
			for (const requestId of requests) {
				const pending = this.#pendingRequests.get(requestId);
				if (pending) {
					pending.reject(new Error(`Actor ${actorId} stopped`));
					this.#pendingRequests.delete(requestId);
				}
			}
			this.#actorRequests.delete(actorId);
		}
		
		// Close all WebSockets for this actor
		const webSockets = this.#actorWebSockets.get(actorId);
		if (webSockets) {
			for (const webSocketId of webSockets) {
				const ws = this.#webSockets.get(webSocketId);
				if (ws) {
					ws.close(1000, "Actor stopped");
					this.#webSockets.delete(webSocketId);
				}
			}
			this.#actorWebSockets.delete(actorId);
		}
	}

	async #fetch(actorId: string, request: Request): Promise<Response> {
		// Validate actor exists
		if (!this.#activeActors.has(actorId)) {
			console.warn(`[TUNNEL] Ignoring request for unknown actor: ${actorId}`);
			return new Response("Actor not found", { status: 404 });
		}
		
		if (!this.#fetchHandler) {
			return new Response("Not Implemented", { status: 501 });
		}
		
		return this.#fetchHandler(actorId, request);
	}

	#connect() {
		if (this.#shutdown) return;

		try {
			this.#ws = new WebSocket(this.#pegboardTunnelUrl, {
				headers: {
					"x-rivet-target": "tunnel",
				},
			});

			this.#ws.binaryType = "arraybuffer";

			this.#ws.addEventListener("open", () => {
				this.#reconnectAttempt = 0;
				
				if (this.#reconnectTimeout) {
					clearTimeout(this.#reconnectTimeout);
					this.#reconnectTimeout = undefined;
				}

				this.#onConnected?.();
			});

			this.#ws.addEventListener("message", async (event) => {
				try {
					await this.#handleMessage(event.data as ArrayBuffer);
				} catch (error) {
					console.error("Error handling tunnel message:", error);
				}
			});

			this.#ws.addEventListener("error", (event) => {
				console.error("Tunnel WebSocket error:", event);
			});

			this.#ws.addEventListener("close", () => {
				this.#onDisconnected?.();

				if (!this.#shutdown) {
					this.#scheduleReconnect();
				}
			});
		} catch (error) {
			console.error("Failed to connect tunnel:", error);
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
		const message = tunnel.decodeTunnelMessage(new Uint8Array(data));
		
		switch (message.body.tag) {
			case "ToServerRequestStart":
				await this.#handleRequestStart(message.body.val);
				break;
			case "ToServerRequestChunk":
				await this.#handleRequestChunk(message.body.val);
				break;
			case "ToServerRequestFinish":
				await this.#handleRequestFinish(message.body.val);
				break;
			case "ToServerWebSocketOpen":
				await this.#handleWebSocketOpen(message.body.val);
				break;
			case "ToServerWebSocketMessage":
				await this.#handleWebSocketMessage(message.body.val);
				break;
			case "ToServerWebSocketClose":
				await this.#handleWebSocketClose(message.body.val);
				break;
			case "ToClientResponseStart":
				this.#handleResponseStart(message.body.val);
				break;
			case "ToClientResponseChunk":
				this.#handleResponseChunk(message.body.val);
				break;
			case "ToClientResponseFinish":
				this.#handleResponseFinish(message.body.val);
				break;
			case "ToClientWebSocketOpen":
				this.#handleWebSocketOpenResponse(message.body.val);
				break;
			case "ToClientWebSocketMessage":
				this.#handleWebSocketMessageResponse(message.body.val);
				break;
			case "ToClientWebSocketClose":
				this.#handleWebSocketCloseResponse(message.body.val);
				break;
		}
	}

	async #handleRequestStart(req: tunnel.ToServerRequestStart) {
		// Track this request for the actor
		const requests = this.#actorRequests.get(req.actorId);
		if (requests) {
			requests.add(req.requestId);
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
						const existing = this.#pendingRequests.get(req.requestId);
						if (existing) {
							existing.streamController = controller;
							existing.actorId = req.actorId;
						} else {
							this.#pendingRequests.set(req.requestId, {
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
				const response = await this.#fetch(req.actorId, streamingRequest);
				await this.#sendResponse(req.requestId, response);
			} else {
				// Non-streaming request
				const response = await this.#fetch(req.actorId, request);
				await this.#sendResponse(req.requestId, response);
			}
		} catch (error) {
			console.error("Error handling request:", error);
			this.#sendResponseError(req.requestId, 500, "Internal Server Error");
		} finally {
			// Clean up request tracking
			const requests = this.#actorRequests.get(req.actorId);
			if (requests) {
				requests.delete(req.requestId);
			}
		}
	}

	async #handleRequestChunk(chunk: tunnel.ToServerRequestChunk) {
		const pending = this.#pendingRequests.get(chunk.requestId);
		if (pending?.streamController) {
			pending.streamController.enqueue(new Uint8Array(chunk.body));
		}
	}

	async #handleRequestFinish(finish: tunnel.ToServerRequestFinish) {
		const pending = this.#pendingRequests.get(finish.requestId);
		if (pending?.streamController) {
			if (finish.reason === tunnel.StreamFinishReason.Complete) {
				pending.streamController.close();
			} else {
				pending.streamController.error(new Error("Request aborted"));
			}
		}
		this.#pendingRequests.delete(finish.requestId);
	}

	async #sendResponse(requestId: bigint, response: Response) {
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
		this.#send({
			body: {
				tag: "ToClientResponseStart",
				val: {
					requestId,
					status: response.status as tunnel.u16,
					headers,
					body: body || null,
					stream: false,
				},
			},
		});
	}

	#sendResponseError(requestId: bigint, status: number, message: string) {
		const headers = new Map<string, string>();
		headers.set("content-type", "text/plain");

		this.#send({
			body: {
				tag: "ToClientResponseStart",
				val: {
					requestId,
					status: status as tunnel.u16,
					headers,
					body: new TextEncoder().encode(message).buffer as ArrayBuffer,
					stream: false,
				},
			},
		});
	}

	async #handleWebSocketOpen(open: tunnel.ToServerWebSocketOpen) {
		// Validate actor exists
		if (!this.#activeActors.has(open.actorId)) {
			console.warn(`Ignoring WebSocket for unknown actor: ${open.actorId}`);
			// Send close immediately
			this.#send({
				body: {
					tag: "ToClientWebSocketClose",
					val: {
						webSocketId: open.webSocketId,
						code: 1011,
						reason: "Actor not found",
					},
				},
			});
			return;
		}

		if (!this.#websocketHandler) {
			console.error("No websocket handler configured for tunnel");
			// Send close immediately
			this.#send({
				body: {
					tag: "ToClientWebSocketClose",
					val: {
						webSocketId: open.webSocketId,
						code: 1011,
						reason: "Not Implemented",
					},
				},
			});
			return;
		}

		// Track this WebSocket for the actor
		const webSockets = this.#actorWebSockets.get(open.actorId);
		if (webSockets) {
			webSockets.add(open.webSocketId);
		}

		try {
			// Create WebSocket adapter
			const adapter = new WebSocketTunnelAdapter(
				open.webSocketId,
				(data: ArrayBuffer | string, isBinary: boolean) => {
					// Send message through tunnel
					const dataBuffer = typeof data === "string" 
						? new TextEncoder().encode(data).buffer as ArrayBuffer
						: data;
					
					this.#send({
						body: {
							tag: "ToClientWebSocketMessage",
							val: {
								webSocketId: open.webSocketId,
								data: dataBuffer,
								binary: isBinary,
							},
						},
					});
				},
				(code?: number, reason?: string) => {
					// Send close through tunnel
					this.#send({
						body: {
							tag: "ToClientWebSocketClose",
							val: {
								webSocketId: open.webSocketId,
								code: code || null,
								reason: reason || null,
							},
						},
					});
					
					// Remove from map
					this.#webSockets.delete(open.webSocketId);
					
					// Clean up actor tracking
					const webSockets = this.#actorWebSockets.get(open.actorId);
					if (webSockets) {
						webSockets.delete(open.webSocketId);
					}
				}
			);

			// Store adapter
			this.#webSockets.set(open.webSocketId, adapter);

			// Send open confirmation
			this.#send({
				body: {
					tag: "ToClientWebSocketOpen",
					val: {
						webSocketId: open.webSocketId,
					},
				},
			});

			// Notify adapter that connection is open
			adapter._handleOpen();

			// Create a minimal request object for the websocket handler
			// Include original headers from the open message
			const headerInit: Record<string, string> = {};
			if (open.headers) {
				for (const [k, v] of open.headers as ReadonlyMap<string, string>) {
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
			await this.#websocketHandler(open.actorId, adapter, request);
		} catch (error) {
			console.error("Error handling WebSocket open:", error);
			
			// Send close on error
			this.#send({
				body: {
					tag: "ToClientWebSocketClose",
					val: {
						webSocketId: open.webSocketId,
						code: 1011,
						reason: "Server Error",
					},
				},
			});
			
			this.#webSockets.delete(open.webSocketId);
			
			// Clean up actor tracking
			const webSockets = this.#actorWebSockets.get(open.actorId);
			if (webSockets) {
				webSockets.delete(open.webSocketId);
			}
		}
	}

	async #handleWebSocketMessage(msg: tunnel.ToServerWebSocketMessage) {
		const adapter = this.#webSockets.get(msg.webSocketId);
		if (adapter) {
			const data = msg.binary
				? new Uint8Array(msg.data)
				: new TextDecoder().decode(new Uint8Array(msg.data));
			
			adapter._handleMessage(data, msg.binary);
		}
	}

	async #handleWebSocketClose(close: tunnel.ToServerWebSocketClose) {
		const adapter = this.#webSockets.get(close.webSocketId);
		if (adapter) {
			adapter._handleClose(close.code || undefined, close.reason || undefined);
			this.#webSockets.delete(close.webSocketId);
		}
	}

	#handleResponseStart(resp: tunnel.ToClientResponseStart) {
		const pending = this.#pendingRequests.get(resp.requestId);
		if (!pending) {
			console.warn(`Received response for unknown request ${resp.requestId}`);
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
			this.#pendingRequests.delete(resp.requestId);
		}
	}

	#handleResponseChunk(chunk: tunnel.ToClientResponseChunk) {
		const pending = this.#pendingRequests.get(chunk.requestId);
		if (pending?.streamController) {
			pending.streamController.enqueue(new Uint8Array(chunk.body));
		}
	}

	#handleResponseFinish(finish: tunnel.ToClientResponseFinish) {
		const pending = this.#pendingRequests.get(finish.requestId);
		if (pending?.streamController) {
			if (finish.reason === tunnel.StreamFinishReason.Complete) {
				pending.streamController.close();
			} else {
				pending.streamController.error(new Error("Response aborted"));
			}
		}
		this.#pendingRequests.delete(finish.requestId);
	}

	#handleWebSocketOpenResponse(open: tunnel.ToClientWebSocketOpen) {
		const adapter = this.#webSockets.get(open.webSocketId);
		if (adapter) {
			adapter._handleOpen();
		}
	}

	#handleWebSocketMessageResponse(msg: tunnel.ToClientWebSocketMessage) {
		const adapter = this.#webSockets.get(msg.webSocketId);
		if (adapter) {
			const data = msg.binary
				? new Uint8Array(msg.data)
				: new TextDecoder().decode(new Uint8Array(msg.data));
			
			adapter._handleMessage(data, msg.binary);
		}
	}

	#handleWebSocketCloseResponse(close: tunnel.ToClientWebSocketClose) {
		const adapter = this.#webSockets.get(close.webSocketId);
		if (adapter) {
			adapter._handleClose(close.code || undefined, close.reason || undefined);
			this.#webSockets.delete(close.webSocketId);
		}
	}

	#send(message: tunnel.TunnelMessage) {
		if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
			console.warn("Cannot send tunnel message, WebSocket not connected");
			return;
		}

		const encoded = tunnel.encodeTunnelMessage(message);
		this.#ws.send(encoded);
	}
}
