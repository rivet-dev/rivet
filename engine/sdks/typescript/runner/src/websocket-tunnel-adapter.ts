// WebSocket-like adapter for tunneled connections
// Implements a subset of the WebSocket interface for compatibility with runner code

import type { Logger } from "pino";
import { logger } from "./log";
import type { Tunnel } from "./tunnel";
import { wrappingAddU16, wrappingLteU16, wrappingSubU16 } from "./utils";

export const HIBERNATABLE_SYMBOL = Symbol("hibernatable");

export class WebSocketTunnelAdapter {
	// MARK: - WebSocket Compat Variables
	#readyState: number = 0; // CONNECTING
	#eventListeners: Map<string, Set<(event: any) => void>> = new Map();
	#onopen: ((this: any, ev: any) => any) | null = null;
	#onclose: ((this: any, ev: any) => any) | null = null;
	#onerror: ((this: any, ev: any) => any) | null = null;
	#onmessage: ((this: any, ev: any) => any) | null = null;
	#bufferedAmount = 0;
	#binaryType: "nodebuffer" | "arraybuffer" | "blob" = "nodebuffer";
	#extensions = "";
	#protocol = "";
	#url = "";

	// mARK: - Internal State
	#tunnel: Tunnel;
	#actorId: string;
	#requestId: string;
	#hibernatable: boolean;
	#messageIndex: number;

	get [HIBERNATABLE_SYMBOL](): boolean {
		return this.#hibernatable;
	}

	/**
	 * Called when sending a message from this WebSocket.
	 *
	 * Used to send a tunnel message from Tunnel.
	 */
	#sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void;

	/**
	 * Called when closing this WebSocket.
	 *
	 * Used to send a tunnel message from Tunnel
	 */
	#closeCallback: (
		code?: number,
		reason?: string,
		hibernate?: boolean,
	) => void;

	get #log(): Logger | undefined {
		return this.#tunnel.log;
	}

	constructor(
		tunnel: Tunnel,
		actorId: string,
		requestId: string,
		hibernatable: boolean,
		messageIndex: number,
		isRestoringHibernatable: boolean,
		/** @experimental */
		public readonly request: Request,
		sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void,
		closeCallback: (code?: number, reason?: string) => void,
	) {
		this.#tunnel = tunnel;
		this.#actorId = actorId;
		this.#requestId = requestId;
		this.#hibernatable = hibernatable;
		this.#messageIndex = messageIndex;
		this.#sendCallback = sendCallback;
		this.#closeCallback = closeCallback;

		// For restored WebSockets, immediately set to OPEN state
		if (isRestoringHibernatable) {
			this.#log?.debug({
				msg: "setting WebSocket to OPEN state for restored connection",
				actorId: this.#actorId,
				requestId: this.#requestId,
				hibernatable: this.#hibernatable,
			});
			this.#readyState = 1; // OPEN
		}
	}

	// MARK: - Lifecycle
	get bufferedAmount(): number {
		return this.#bufferedAmount;
	}

	_handleOpen(requestId: ArrayBuffer): void {
		if (this.#readyState !== 0) {
			// CONNECTING
			return;
		}

		this.#readyState = 1; // OPEN

		const event = {
			type: "open",
			rivetRequestId: requestId,
			target: this,
		};

		this.#fireEvent("open", event);
	}

	_handleMessage(
		requestId: ArrayBuffer,
		data: string | Uint8Array,
		messageIndex: number,
		isBinary: boolean,
	): boolean {
		if (this.#readyState !== 1) {
			this.#log?.warn({
				msg: "WebSocket message ignored - not in OPEN state",
				requestId: this.#requestId,
				actorId: this.#actorId,
				currentReadyState: this.#readyState,
				expectedReadyState: 1,
				messageIndex,
				hibernatable: this.#hibernatable,
			});
			return true;
		}

		// Validate message index
		if (this.#hibernatable) {
			const previousIndex = this.#messageIndex;

			// Ignore duplicate old messages
			//
			// This should only happen if something goes wrong
			// between persisting the previous index and acking the
			// message index to the gateway. If the ack is never
			// received by the gateway (due to a crash or network
			// issue), the gateway will resend all messages from
			// the last ack on reconnect.
			if (wrappingLteU16(messageIndex, previousIndex)) {
				this.#log?.info({
					msg: "received duplicate hibernating websocket message, this indicates the actor failed to ack the message index before restarting",
					requestId,
					actorId: this.#actorId,
					previousIndex,
					expectedIndex: wrappingAddU16(previousIndex, 1),
					receivedIndex: messageIndex,
				});

				return true;
			}

			// Close message if skipped message in sequence
			//
			// There is no scenario where this should ever happen
			const expectedIndex = wrappingAddU16(previousIndex, 1);
			if (messageIndex !== expectedIndex) {
				const closeReason = "ws.message_index_skip";

				this.#log?.warn({
					msg: "hibernatable websocket message index out of sequence, closing connection",
					requestId,
					actorId: this.#actorId,
					previousIndex,
					expectedIndex,
					receivedIndex: messageIndex,
					closeReason,
					gap: wrappingSubU16(
						wrappingSubU16(messageIndex, previousIndex),
						1,
					),
				});

				// Close the WebSocket and skip processing
				this.close(1008, closeReason);

				return true;
			}

			// Update to the next index
			this.#messageIndex = messageIndex;
		}

		// Dispatch event
		let messageData: any;
		if (isBinary) {
			// Handle binary data based on binaryType
			if (this.#binaryType === "nodebuffer") {
				// Convert to Buffer for Node.js compatibility
				messageData = Buffer.from(data as Uint8Array);
			} else if (this.#binaryType === "arraybuffer") {
				// Convert to ArrayBuffer
				if (data instanceof Uint8Array) {
					messageData = data.buffer.slice(
						data.byteOffset,
						data.byteOffset + data.byteLength,
					);
				} else {
					messageData = data;
				}
			} else {
				// Blob type - not commonly used in Node.js
				throw new Error(
					"Blob binaryType not supported in tunnel adapter",
				);
			}
		} else {
			messageData = data;
		}

		const event = {
			type: "message",
			data: messageData,
			rivetRequestId: requestId,
			rivetMessageIndex: messageIndex,
			target: this,
		};

		this.#fireEvent("message", event);

		return false;
	}

	_handleClose(
		_requestId: ArrayBuffer,
		code?: number,
		reason?: string,
	): void {
		this.#closeInner(code, reason, true);
	}

	_handleError(error: Error): void {
		const event = {
			type: "error",
			target: this,
			error,
		};

		this.#fireEvent("error", event);
	}

	_closeWithoutCallback(code?: number, reason?: string): void {
		this.#closeInner(code, reason, false);
	}

	#fireEvent(type: string, event: any): void {
		// Call all registered event listeners
		const listeners = this.#eventListeners.get(type);

		if (listeners && listeners.size > 0) {
			for (const listener of listeners) {
				try {
					listener.call(this, event);
				} catch (error) {
					logger()?.error({
						msg: "error in websocket event listener",
						error,
						type,
					});
				}
			}
		}

		// Call the onX property if set
		switch (type) {
			case "open":
				if (this.#onopen) {
					try {
						this.#onopen.call(this, event);
					} catch (error) {
						logger()?.error({
							msg: "error in onopen handler",
							error,
						});
					}
				}
				break;
			case "close":
				if (this.#onclose) {
					try {
						this.#onclose.call(this, event);
					} catch (error) {
						logger()?.error({
							msg: "error in onclose handler",
							error,
						});
					}
				}
				break;
			case "error":
				if (this.#onerror) {
					try {
						this.#onerror.call(this, event);
					} catch (error) {
						logger()?.error({
							msg: "error in onerror handler",
							error,
						});
					}
				}
				break;
			case "message":
				if (this.#onmessage) {
					try {
						this.#onmessage.call(this, event);
					} catch (error) {
						logger()?.error({
							msg: "error in onmessage handler",
							error,
						});
					}
				}
				break;
		}
	}

	#closeInner(
		code: number | undefined,
		reason: string | undefined,
		callback: boolean,
	): void {
		if (
			this.#readyState === 2 || // CLOSING
			this.#readyState === 3 // CLOSED
		) {
			return;
		}

		this.#readyState = 2; // CLOSING

		// Send close through tunnel
		if (callback) {
			this.#closeCallback(code, reason);
		}

		// Update state and fire event
		this.#readyState = 3; // CLOSED

		const closeEvent = {
			wasClean: true,
			code: code || 1000,
			reason: reason || "",
			type: "close",
			target: this,
		};

		this.#fireEvent("close", closeEvent);
	}

	// MARK: - WebSocket Compatible API
	get readyState(): number {
		return this.#readyState;
	}

	get binaryType(): string {
		return this.#binaryType;
	}

	set binaryType(value: string) {
		if (
			value === "nodebuffer" ||
			value === "arraybuffer" ||
			value === "blob"
		) {
			this.#binaryType = value;
		}
	}

	get extensions(): string {
		return this.#extensions;
	}

	get protocol(): string {
		return this.#protocol;
	}

	get url(): string {
		return this.#url;
	}

	get onopen(): ((this: any, ev: any) => any) | null {
		return this.#onopen;
	}

	set onopen(value: ((this: any, ev: any) => any) | null) {
		this.#onopen = value;
	}

	get onclose(): ((this: any, ev: any) => any) | null {
		return this.#onclose;
	}

	set onclose(value: ((this: any, ev: any) => any) | null) {
		this.#onclose = value;
	}

	get onerror(): ((this: any, ev: any) => any) | null {
		return this.#onerror;
	}

	set onerror(value: ((this: any, ev: any) => any) | null) {
		this.#onerror = value;
	}

	get onmessage(): ((this: any, ev: any) => any) | null {
		return this.#onmessage;
	}

	set onmessage(value: ((this: any, ev: any) => any) | null) {
		this.#onmessage = value;
	}

	send(data: string | ArrayBuffer | ArrayBufferView | Blob | Buffer): void {
		// Handle different ready states
		if (this.#readyState === 0) {
			// CONNECTING
			throw new DOMException(
				"WebSocket is still in CONNECTING state",
				"InvalidStateError",
			);
		}

		if (this.#readyState === 2 || this.#readyState === 3) {
			// CLOSING or CLOSED - silently ignore
			return;
		}

		let isBinary = false;
		let messageData: string | ArrayBuffer;

		if (typeof data === "string") {
			messageData = data;
		} else if (data instanceof ArrayBuffer) {
			isBinary = true;
			messageData = data;
		} else if (ArrayBuffer.isView(data)) {
			isBinary = true;
			// Convert ArrayBufferView to ArrayBuffer
			const view = data as ArrayBufferView;
			// Check if it's a SharedArrayBuffer
			if (view.buffer instanceof SharedArrayBuffer) {
				// Copy SharedArrayBuffer to regular ArrayBuffer
				const bytes = new Uint8Array(
					view.buffer,
					view.byteOffset,
					view.byteLength,
				);
				messageData = bytes.buffer.slice(
					bytes.byteOffset,
					bytes.byteOffset + bytes.byteLength,
				) as unknown as ArrayBuffer;
			} else {
				messageData = view.buffer.slice(
					view.byteOffset,
					view.byteOffset + view.byteLength,
				) as ArrayBuffer;
			}
		} else if (data instanceof Blob) {
			throw new Error("Blob sending not implemented in tunnel adapter");
		} else if (typeof Buffer !== "undefined" && Buffer.isBuffer(data)) {
			isBinary = true;
			// Convert Buffer to ArrayBuffer
			const buf = data as Buffer;
			// Check if it's a SharedArrayBuffer
			if (buf.buffer instanceof SharedArrayBuffer) {
				// Copy SharedArrayBuffer to regular ArrayBuffer
				const bytes = new Uint8Array(
					buf.buffer,
					buf.byteOffset,
					buf.byteLength,
				);
				messageData = bytes.buffer.slice(
					bytes.byteOffset,
					bytes.byteOffset + bytes.byteLength,
				) as unknown as ArrayBuffer;
			} else {
				messageData = buf.buffer.slice(
					buf.byteOffset,
					buf.byteOffset + buf.byteLength,
				) as ArrayBuffer;
			}
		} else {
			throw new Error("Invalid data type");
		}

		// Send through tunnel
		this.#sendCallback(messageData, isBinary);
	}

	close(code?: number, reason?: string): void {
		this.#closeInner(code, reason, true);
	}

	addEventListener(
		type: string,
		listener: (event: any) => void,
		options?: boolean | any,
	): void {
		if (typeof listener === "function") {
			let listeners = this.#eventListeners.get(type);
			if (!listeners) {
				listeners = new Set();
				this.#eventListeners.set(type, listeners);
			}
			listeners.add(listener);
		}
	}

	removeEventListener(
		type: string,
		listener: (event: any) => void,
		options?: boolean | any,
	): void {
		if (typeof listener === "function") {
			const listeners = this.#eventListeners.get(type);
			if (listeners) {
				listeners.delete(listener);
			}
		}
	}

	dispatchEvent(event: any): boolean {
		// TODO:
		return true;
	}

	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	readonly CONNECTING = 0;
	readonly OPEN = 1;
	readonly CLOSING = 2;
	readonly CLOSED = 3;

	// Additional methods for compatibility
	ping(data?: any, mask?: boolean, cb?: (err: Error) => void): void {
		// Not implemented for tunnel - could be added if needed
		if (cb) cb(new Error("Ping not supported in tunnel adapter"));
	}

	pong(data?: any, mask?: boolean, cb?: (err: Error) => void): void {
		// Not implemented for tunnel - could be added if needed
		if (cb) cb(new Error("Pong not supported in tunnel adapter"));
	}

	/** @experimental */
	terminate(): void {
		// Immediate close without close frame
		this.#readyState = 3; // CLOSED
		this.#closeCallback(1006, "Abnormal Closure");

		const event = {
			wasClean: false,
			code: 1006,
			reason: "Abnormal Closure",
			type: "close",
			target: this,
		};

		this.#fireEvent("close", event);
	}
}
