import { WSContext } from "hono/ws";
import type { UpgradeWebSocketArgs } from "@/actor/router-websocket-endpoints";
import type {
	RivetCloseEvent,
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import { getLogger } from "./log";

export function logger() {
	return getLogger("fake-event-source2");
}

/**
 * InlineWebSocketAdapter implements a WebSocket-like interface
 * that connects to a UpgradeWebSocketArgs handler
 */
export class InlineWebSocketAdapter implements UniversalWebSocket {
	// WebSocket readyState values
	readonly CONNECTING = 0 as const;
	readonly OPEN = 1 as const;
	readonly CLOSING = 2 as const;
	readonly CLOSED = 3 as const;

	// Private properties
	#handler: UpgradeWebSocketArgs;
	#wsContext: WSContext;
	#readyState: 0 | 1 | 2 | 3 = 0; // Start in CONNECTING state
	#queuedMessages: Array<string | ArrayBuffer | Uint8Array> = [];

	// Event listeners
	#eventListeners: Map<string, ((ev: any) => void)[]> = new Map();

	constructor(handler: UpgradeWebSocketArgs) {
		this.#handler = handler;

		// Create a fake WSContext to pass to the handler
		this.#wsContext = new WSContext({
			raw: this,
			send: (data: string | ArrayBuffer | Uint8Array) => {
				logger().debug({ msg: "WSContext.send called" });
				this.#handleMessage(data);
			},
			close: (code?: number, reason?: string) => {
				logger().debug({ msg: "WSContext.close called", code, reason });
				this.#handleClose(code || 1000, reason || "");
			},
			// Set readyState to 1 (OPEN) since handlers expect an open connection
			readyState: 1,
		});

		// Set __adapter on WSContext so handleRawWebSocket.onMessage can route messages back
		(this.#wsContext as any).__adapter = this;

		// Initialize the connection
		//
		// Defer initialization to allow event listeners to be attached first
		setTimeout(() => {
			this.#initialize();
		}, 0);
	}

	get readyState(): 0 | 1 | 2 | 3 {
		return this.#readyState;
	}

	get binaryType(): "arraybuffer" | "blob" {
		return "arraybuffer";
	}

	set binaryType(value: "arraybuffer" | "blob") {
		// Ignored for now - always use arraybuffer
	}

	get bufferedAmount(): number {
		return 0; // Not tracked in InlineWebSocketAdapter
	}

	get extensions(): string {
		return ""; // Not available in InlineWebSocketAdapter
	}

	get protocol(): string {
		return ""; // Not available in InlineWebSocketAdapter
	}

	get url(): string {
		return ""; // Not available in InlineWebSocketAdapter
	}

	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		logger().debug({ msg: "send called", readyState: this.readyState });

		// Handle different ready states
		if (this.readyState === this.CONNECTING) {
			// Throw InvalidStateError if still connecting
			throw new DOMException(
				"WebSocket is still in CONNECTING state",
				"InvalidStateError",
			);
		}

		if (
			this.readyState === this.CLOSING ||
			this.readyState === this.CLOSED
		) {
			// Silently ignore if closing or closed
			logger().debug({
				msg: "ignoring send, websocket is closing or closed",
				readyState: this.readyState,
			});
			return;
		}

		// Must be OPEN at this point
		this.#handler.onMessage({ data }, this.#wsContext);
	}

	/**
	 * Closes the connection
	 */
	close(code = 1000, reason = ""): void {
		if (
			this.readyState === this.CLOSED ||
			this.readyState === this.CLOSING
		) {
			return;
		}

		logger().debug({ msg: "closing fake websocket", code, reason });

		this.#readyState = this.CLOSING;

		// Call the handler's onClose method
		try {
			this.#handler.onClose(
				{ code, reason, wasClean: true },
				this.#wsContext,
			);
		} catch (err) {
			logger().error({ msg: "error closing websocket", error: err });
		} finally {
			this.#readyState = this.CLOSED;

			// Fire the close event
			// Create a close event object since CloseEvent is not available in Node.js
			const closeEvent = {
				type: "close",
				wasClean: code === 1000,
				code,
				reason,
				target: this,
				currentTarget: this,
			} as unknown as RivetCloseEvent;

			this.#fireClose(closeEvent);
		}
	}

	/**
	 * Initialize the connection with the handler
	 */
	async #initialize(): Promise<void> {
		try {
			logger().debug({ msg: "fake websocket initializing" });

			// Call the handler's onOpen method
			logger().debug({ msg: "calling handler.onOpen with WSContext" });
			this.#readyState = this.OPEN;
			this.#handler.onOpen(undefined, this.#wsContext);

			// Fire the open event
			this.#fireOpen();

			// Delay processing queued messages slightly to allow event handlers to be set up
			if (this.#queuedMessages.length > 0) {
				if (this.readyState !== this.OPEN) {
					logger().warn({
						msg: "socket no longer open, dropping queued messages",
					});
					return;
				}

				logger().debug({
					msg: `now processing ${this.#queuedMessages.length} queued messages`,
				});

				// Create a copy to avoid issues if new messages arrive during processing
				const messagesToProcess = [...this.#queuedMessages];
				this.#queuedMessages = [];

				// Process each queued message
				for (const message of messagesToProcess) {
					logger().debug({ msg: "processing queued message" });
					this.#handleMessage(message);
				}
			}
		} catch (err) {
			logger().error({
				msg: "error opening fake websocket",
				error: err,
				errorMessage: err instanceof Error ? err.message : String(err),
				stack: err instanceof Error ? err.stack : undefined,
			});
			this.#fireError(err);
			this.close(1011, "Internal error during initialization");
		}
	}
	/**
	 * Handle message from actor (called via __adapter from handleRawWebSocket.onMessage)
	 */
	_handleMessage(event: MessageEvent) {
	  this.#handleMessage(event.data);
	}

	/**
	 * Handle messages received from the server via the WSContext
	 */
	#handleMessage(data: string | ArrayBuffer | Uint8Array): void {
		// Store messages that arrive before the socket is fully initialized
		if (this.readyState !== this.OPEN) {
			logger().debug({
				msg: "message received before socket is OPEN, queuing",
				readyState: this.readyState,
				dataType: typeof data,
				dataLength:
					typeof data === "string"
						? data.length
						: data instanceof ArrayBuffer
							? data.byteLength
							: data instanceof Uint8Array
								? data.byteLength
								: "unknown",
			});

			// Queue the message to be processed once the socket is open
			this.#queuedMessages.push(data);
			return;
		}

		// Log message received from server
		logger().debug({
			msg: "fake websocket received message from server",
			dataType: typeof data,
			dataLength:
				typeof data === "string"
					? data.length
					: data instanceof ArrayBuffer
						? data.byteLength
						: data instanceof Uint8Array
							? data.byteLength
							: "unknown",
		});

		// Create a MessageEvent-like object
		const event = {
			type: "message",
			data,
			target: this,
			currentTarget: this,
		} as unknown as RivetMessageEvent;

		// Dispatch the event
		this.#dispatchEvent("message", event);
	}

	#handleClose(code: number, reason: string): void {
		if (this.readyState === this.CLOSED) return;

		this.#readyState = this.CLOSED;

		// Create a CloseEvent-like object
		const event = {
			type: "close",
			code,
			reason,
			wasClean: code === 1000,
			target: this,
			currentTarget: this,
		} as unknown as RivetCloseEvent;

		// Dispatch the event
		this.#dispatchEvent("close", event);
	}

	addEventListener(type: string, listener: (ev: any) => void): void {
		if (!this.#eventListeners.has(type)) {
			this.#eventListeners.set(type, []);
		}
		this.#eventListeners.get(type)!.push(listener);
	}

	removeEventListener(type: string, listener: (ev: any) => void): void {
		const listeners = this.#eventListeners.get(type);
		if (listeners) {
			const index = listeners.indexOf(listener);
			if (index !== -1) {
				listeners.splice(index, 1);
			}
		}
	}

	#dispatchEvent(type: string, event: any): void {
		const listeners = this.#eventListeners.get(type);
		if (listeners && listeners.length > 0) {
			logger().debug(
				`dispatching ${type} event to ${listeners.length} listeners`,
			);
			for (const listener of listeners) {
				try {
					listener(event);
				} catch (err) {
					logger().error({
						msg: `error in ${type} event listener`,
						error: err,
					});
				}
			}
		}

		// Also check for on* properties
		switch (type) {
			case "open":
				if (this.#onopen) {
					try {
						this.#onopen(event);
					} catch (error) {
						logger().error({
							msg: "error in onopen handler",
							error,
						});
					}
				}
				break;
			case "close":
				if (this.#onclose) {
					try {
						this.#onclose(event);
					} catch (error) {
						logger().error({
							msg: "error in onclose handler",
							error,
						});
					}
				}
				break;
			case "error":
				if (this.#onerror) {
					try {
						this.#onerror(event);
					} catch (error) {
						logger().error({
							msg: "error in onerror handler",
							error,
						});
					}
				}
				break;
			case "message":
				if (this.#onmessage) {
					try {
						this.#onmessage(event);
					} catch (error) {
						logger().error({
							msg: "error in onmessage handler",
							error,
						});
					}
				}
				break;
		}
	}

	dispatchEvent(event: RivetEvent): boolean {
		this.#dispatchEvent(event.type, event);
		return true;
	}

	#fireOpen(): void {
		try {
			// Create an Event-like object since Event constructor may not be available
			const event = {
				type: "open",
				target: this,
				currentTarget: this,
			} as unknown as RivetEvent;

			this.#dispatchEvent("open", event);
		} catch (err) {
			logger().error({ msg: "error in open event", error: err });
		}
	}

	#fireClose(event: RivetCloseEvent): void {
		try {
			this.#dispatchEvent("close", event);
		} catch (err) {
			logger().error({ msg: "error in close event", error: err });
		}
	}

	#fireError(error: unknown): void {
		try {
			// Create an Event-like object for error
			const event = {
				type: "error",
				target: this,
				currentTarget: this,
				error,
				message: error instanceof Error ? error.message : String(error),
			} as unknown as RivetEvent;

			this.#dispatchEvent("error", event);
		} catch (err) {
			logger().error({ msg: "error in error event", error: err });
		}

		// Log the error
		logger().error({ msg: "websocket error", error });
	}

	// Event handler properties with getters/setters
	#onopen: ((event: RivetEvent) => void) | null = null;
	#onclose: ((event: RivetCloseEvent) => void) | null = null;
	#onerror: ((event: RivetEvent) => void) | null = null;
	#onmessage: ((event: RivetMessageEvent) => void) | null = null;

	get onopen(): ((event: RivetEvent) => void) | null {
		return this.#onopen;
	}
	set onopen(handler: ((event: RivetEvent) => void) | null) {
		this.#onopen = handler;
	}

	get onclose(): ((event: RivetCloseEvent) => void) | null {
		return this.#onclose;
	}
	set onclose(handler: ((event: RivetCloseEvent) => void) | null) {
		this.#onclose = handler;
	}

	get onerror(): ((event: RivetEvent) => void) | null {
		return this.#onerror;
	}
	set onerror(handler: ((event: RivetEvent) => void) | null) {
		this.#onerror = handler;
	}

	get onmessage(): ((event: RivetMessageEvent) => void) | null {
		return this.#onmessage;
	}
	set onmessage(handler: ((event: RivetMessageEvent) => void) | null) {
		this.#onmessage = handler;
	}
}
