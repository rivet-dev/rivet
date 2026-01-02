import { WSContext } from "hono/ws";
import type { UpgradeWebSocketArgs } from "@/actor/router-websocket-endpoints";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import { VirtualWebSocket } from "@rivetkit/virtual-websocket";
import { getLogger } from "./log";

function logger() {
	return getLogger("inline-websocket-adapter");
}

/**
 * InlineWebSocketAdapter creates two linked WebSocket objects:
 * - clientWs: for the client/proxy side (returned from openWebSocket)
 * - actorWs: for the actor side (passed via wsContext.raw)
 *
 * Each side's send() triggers the OTHER side's message event.
 */
export class InlineWebSocketAdapter {
	#handler: UpgradeWebSocketArgs;
	#wsContext: WSContext;
	#readyState: 0 | 1 | 2 | 3 = 0;

	#clientWs: VirtualWebSocket;
	#actorWs: VirtualWebSocket;

	constructor(handler: UpgradeWebSocketArgs) {
		this.#handler = handler;

		// Create linked WebSocket pair
		// Client's send() -> handler.onMessage (for RPC) + Actor's message event (for raw WS)
		// Actor's send() -> Client's message event
		this.#clientWs = new VirtualWebSocket({
			getReadyState: () => this.#readyState,
			onSend: (data) => {
				try {
					// Call handler.onMessage for protocol-based connections (RPC)
					this.#handler.onMessage({ data }, this.#wsContext);
					// Also trigger message event on actor's websocket for raw websocket handlers
					this.#actorWs.triggerMessage(data);
				} catch (err) {
					this.#handleError(err);
					this.#close(1011, "Internal error processing message");
				}
			},
			onClose: (code, reason) => this.#close(code, reason),
		});

		this.#actorWs = new VirtualWebSocket({
			getReadyState: () => this.#readyState,
			onSend: (data) => this.#clientWs.triggerMessage(data),
			onClose: (code, reason) => this.#close(code, reason),
		});

		// Create WSContext with actorWs as raw
		this.#wsContext = new WSContext({
			raw: this.#actorWs,
			send: (data: string | ArrayBuffer | Uint8Array) => {
				logger().debug({ msg: "WSContext.send called" });
				this.#clientWs.triggerMessage(data);
			},
			close: (code?: number, reason?: string) => {
				logger().debug({ msg: "WSContext.close called", code, reason });
				this.#close(code || 1000, reason || "");
			},
			readyState: 1,
		});

		// Defer initialization to allow event listeners to be attached first
		setTimeout(() => {
			this.#initialize();
		}, 0);
	}

	/** Get the client-side WebSocket (for proxy/client code) */
	get clientWebSocket(): UniversalWebSocket {
		return this.#clientWs;
	}

	/** Get the actor-side WebSocket (passed to actor via wsContext.raw) */
	get actorWebSocket(): UniversalWebSocket {
		return this.#actorWs;
	}

	async #initialize(): Promise<void> {
		try {
			logger().debug({ msg: "websocket initializing" });

			this.#readyState = 1; // OPEN

			logger().debug({ msg: "calling handler.onOpen with WSContext" });
			this.#handler.onOpen(undefined, this.#wsContext);

			// Fire open event to both sides
			this.#clientWs.triggerOpen();
			this.#actorWs.triggerOpen();
		} catch (err) {
			this.#handleError(err);
			this.#close(1011, "Internal error during initialization");
		}
	}

	#handleError(err: unknown): void {
		logger().error({
			msg: "error in websocket",
			error: err,
			errorMessage: err instanceof Error ? err.message : String(err),
			stack: err instanceof Error ? err.stack : undefined,
		});

		// Call handler.onError
		try {
			this.#handler.onError(err, this.#wsContext);
		} catch (handlerErr) {
			logger().error({ msg: "error in onError handler", error: handlerErr });
		}

		// Fire error event to both sides
		this.#clientWs.triggerError(err);
		this.#actorWs.triggerError(err);
	}

	#close(code: number, reason: string): void {
		if (this.#readyState === 3 || this.#readyState === 2) {
			return;
		}

		logger().debug({ msg: "closing websocket", code, reason });

		this.#readyState = 2; // CLOSING

		try {
			this.#handler.onClose({ code, reason, wasClean: true }, this.#wsContext);
		} catch (err) {
			logger().error({ msg: "error closing websocket", error: err });
		} finally {
			this.#readyState = 3; // CLOSED

			// Fire close event to both sides
			this.#clientWs.triggerClose(code, reason);
			this.#actorWs.triggerClose(code, reason);
		}
	}
}

/**
 * Creates an InlineWebSocketAdapter and returns the client-side WebSocket.
 * This is the main entry point for creating inline WebSocket connections.
 */
export function createInlineWebSocket(
	handler: UpgradeWebSocketArgs,
): UniversalWebSocket {
	const adapter = new InlineWebSocketAdapter(handler);
	return adapter.clientWebSocket;
}
