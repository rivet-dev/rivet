import type { WSContext } from "hono/ws";
import type {
	RivetCloseEvent,
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import { logger } from "./log";

/**
 * HonoWebSocketAdapter provides a WebSocket-like interface over WSContext
 * for raw WebSocket handling in actors
 */
export class HonoWebSocketAdapter implements UniversalWebSocket {
	// WebSocket readyState values
	readonly CONNECTING = 0 as const;
	readonly OPEN = 1 as const;
	readonly CLOSING = 2 as const;
	readonly CLOSED = 3 as const;

	#ws: WSContext;
	#readyState: 0 | 1 | 2 | 3 = 1; // Start as OPEN since WSContext is already connected
	#eventListeners: Map<string, Set<(event: any) => void>> = new Map();
	#closeCode?: number;
	#closeReason?: string;
	readonly rivetRequestId?: ArrayBuffer;
	readonly isHibernatable: boolean;

	constructor(
		ws: WSContext,
		rivetRequestId: ArrayBuffer | undefined,
		isHibernatable: boolean,
	) {
		this.#ws = ws;
		this.rivetRequestId = rivetRequestId;
		this.isHibernatable = isHibernatable;

		// The WSContext is already open when we receive it
		this.#readyState = this.OPEN;

		// Fire open event on next tick so the runtime has time to schedule event listeners
		setTimeout(() => {
			this.#fireEvent("open", {
				type: "open",
				target: this,
				rivetRequestId: this.rivetRequestId,
			});
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
		return 0; // Not tracked in WSContext
	}

	get extensions(): string {
		return ""; // Not available in WSContext
	}

	get protocol(): string {
		return ""; // Not available in WSContext
	}

	get url(): string {
		return ""; // Not available in WSContext
	}

	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		if (this.readyState !== this.OPEN) {
			throw new Error("WebSocket is not open");
		}

		try {
			logger().debug({
				msg: "bridge sending data",
				dataType: typeof data,
				isString: typeof data === "string",
				isArrayBuffer: data instanceof ArrayBuffer,
				dataStr:
					typeof data === "string"
						? data.substring(0, 100)
						: "<non-string>",
			});

			if (typeof data === "string") {
				(this.#ws as any).send(data);
			} else if (data instanceof ArrayBuffer) {
				(this.#ws as any).send(data);
			} else if (ArrayBuffer.isView(data)) {
				// Convert ArrayBufferView to ArrayBuffer
				const buffer = data.buffer.slice(
					data.byteOffset,
					data.byteOffset + data.byteLength,
				);
				// Check if it's a SharedArrayBuffer and convert to ArrayBuffer
				if (buffer instanceof SharedArrayBuffer) {
					const arrayBuffer = new ArrayBuffer(buffer.byteLength);
					new Uint8Array(arrayBuffer).set(new Uint8Array(buffer));
					(this.#ws as any).send(arrayBuffer);
				} else {
					(this.#ws as any).send(buffer);
				}
			} else if (data instanceof Blob) {
				// Convert Blob to ArrayBuffer
				data.arrayBuffer()
					.then((buffer) => {
						(this.#ws as any).send(buffer);
					})
					.catch((error) => {
						logger().error({
							msg: "failed to convert blob to arraybuffer",
							error,
						});
						this.#fireEvent("error", {
							type: "error",
							target: this,
							error,
						});
					});
			} else {
				// Try to convert to string as a fallback
				logger().warn({
					msg: "unsupported data type, converting to string",
					dataType: typeof data,
					data,
				});
				(this.#ws as any).send(String(data));
			}
		} catch (error) {
			logger().error({ msg: "error sending websocket data", error });
			this.#fireEvent("error", { type: "error", target: this, error });
			throw error;
		}
	}

	close(code = 1000, reason = ""): void {
		if (
			this.readyState === this.CLOSING ||
			this.readyState === this.CLOSED
		) {
			return;
		}

		this.#readyState = this.CLOSING;
		this.#closeCode = code;
		this.#closeReason = reason;

		try {
			(this.#ws as any).close(code, reason);

			// Update state and fire close event
			this.#readyState = this.CLOSED;
			this.#fireEvent("close", {
				type: "close",
				target: this,
				code,
				reason,
				wasClean: code === 1000,
				rivetRequestId: this.rivetRequestId,
			});
		} catch (error) {
			logger().error({ msg: "error closing websocket", error });
			this.#readyState = this.CLOSED;
			this.#fireEvent("close", {
				type: "close",
				target: this,
				code: 1006,
				reason: "Abnormal closure",
				wasClean: false,
				rivetRequestId: this.rivetRequestId,
			});
		}
	}

	addEventListener(type: string, listener: (event: any) => void): void {
		if (!this.#eventListeners.has(type)) {
			this.#eventListeners.set(type, new Set());
		}
		this.#eventListeners.get(type)!.add(listener);
	}

	removeEventListener(type: string, listener: (event: any) => void): void {
		const listeners = this.#eventListeners.get(type);
		if (listeners) {
			listeners.delete(listener);
		}
	}

	dispatchEvent(event: RivetEvent): boolean {
		const listeners = this.#eventListeners.get(event.type);
		if (listeners) {
			for (const listener of listeners) {
				try {
					listener(event);
				} catch (error) {
					logger().error({
						msg: `error in ${event.type} event listener`,
						error,
					});
				}
			}
		}
		return true;
	}

	// Internal method to handle incoming messages from WSContext
	_handleMessage(data: any): void {
		// Hono may pass either raw data or a MessageEvent-like object
		let messageData: string | ArrayBuffer | ArrayBufferView;
		let rivetRequestId: ArrayBuffer | undefined;
		let rivetMessageIndex: number | undefined;

		if (typeof data === "string") {
			messageData = data;
		} else if (data instanceof ArrayBuffer || ArrayBuffer.isView(data)) {
			messageData = data;
		} else if (data && typeof data === "object" && "data" in data) {
			// Handle MessageEvent-like objects
			messageData = data.data;

			// Preserve hibernation-related properties from engine runner
			if ("rivetRequestId" in data) {
				rivetRequestId = data.rivetRequestId;
			}
			if ("rivetMessageIndex" in data) {
				rivetMessageIndex = data.rivetMessageIndex;
			}
		} else {
			// Fallback - shouldn't happen in normal operation
			messageData = String(data);
		}

		logger().debug({
			msg: "bridge handling message",
			dataType: typeof messageData,
			isArrayBuffer: messageData instanceof ArrayBuffer,
			dataStr: typeof messageData === "string" ? messageData : "<binary>",
			rivetMessageIndex,
		});

		this.#fireEvent("message", {
			type: "message",
			target: this,
			data: messageData,
			rivetRequestId,
			rivetMessageIndex,
		});
	}

	// Internal method to handle close from WSContext
	_handleClose(code: number, reason: string): void {
		// HACK: Close socket in order to fix bug with Cloudflare leaving WS in closing state
		// https://github.com/cloudflare/workerd/issues/2569
		(this.#ws as any).close(1000, "hack_force_close");

		if (this.readyState === this.CLOSED) return;

		this.#readyState = this.CLOSED;
		this.#closeCode = code;
		this.#closeReason = reason;

		this.#fireEvent("close", {
			type: "close",
			target: this,
			code,
			reason,
			wasClean: code === 1000,
			rivetRequestId: this.rivetRequestId,
		});
	}

	// Internal method to handle errors from WSContext
	_handleError(error: any): void {
		this.#fireEvent("error", {
			type: "error",
			target: this,
			error,
		});
	}

	#fireEvent(type: string, event: any): void {
		const listeners = this.#eventListeners.get(type);
		if (listeners) {
			for (const listener of listeners) {
				try {
					listener(event);
				} catch (error) {
					logger().error({
						msg: `error in ${type} event listener`,
						error,
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
