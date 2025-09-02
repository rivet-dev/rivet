// WebSocket-like adapter for tunneled connections
// Implements a subset of the WebSocket interface for compatibility with runner code

export class WebSocketTunnelAdapter {
	#webSocketId: bigint;
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
	#sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void;
	#closeCallback: (code?: number, reason?: string) => void;
	
	// Event buffering for events fired before listeners are attached
	#bufferedEvents: Array<{
		type: string;
		event: any;
	}> = [];

	constructor(
		webSocketId: bigint,
		sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void,
		closeCallback: (code?: number, reason?: string) => void
	) {
		this.#webSocketId = webSocketId;
		this.#sendCallback = sendCallback;
		this.#closeCallback = closeCallback;
	}

	get readyState(): number {
		return this.#readyState;
	}

	get bufferedAmount(): number {
		return this.#bufferedAmount;
	}

	get binaryType(): string {
		return this.#binaryType;
	}

	set binaryType(value: string) {
		if (value === "nodebuffer" || value === "arraybuffer" || value === "blob") {
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
		// Flush any buffered open events when onopen is set
		if (value) {
			this.#flushBufferedEvents("open");
		}
	}

	get onclose(): ((this: any, ev: any) => any) | null {
		return this.#onclose;
	}

	set onclose(value: ((this: any, ev: any) => any) | null) {
		this.#onclose = value;
		// Flush any buffered close events when onclose is set
		if (value) {
			this.#flushBufferedEvents("close");
		}
	}

	get onerror(): ((this: any, ev: any) => any) | null {
		return this.#onerror;
	}

	set onerror(value: ((this: any, ev: any) => any) | null) {
		this.#onerror = value;
		// Flush any buffered error events when onerror is set
		if (value) {
			this.#flushBufferedEvents("error");
		}
	}

	get onmessage(): ((this: any, ev: any) => any) | null {
		return this.#onmessage;
	}

	set onmessage(value: ((this: any, ev: any) => any) | null) {
		this.#onmessage = value;
		// Flush any buffered message events when onmessage is set
		if (value) {
			this.#flushBufferedEvents("message");
		}
	}

	send(data: string | ArrayBuffer | ArrayBufferView | Blob | Buffer): void {
		if (this.#readyState !== 1) { // OPEN
			throw new Error("WebSocket is not open");
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
				const bytes = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
				messageData = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as unknown as ArrayBuffer;
			} else {
				messageData = view.buffer.slice(
					view.byteOffset,
					view.byteOffset + view.byteLength
				) as ArrayBuffer;
			}
		} else if (data instanceof Blob) {
			throw new Error("Blob sending not implemented in tunnel adapter");
		} else if (typeof Buffer !== 'undefined' && Buffer.isBuffer(data)) {
			isBinary = true;
			// Convert Buffer to ArrayBuffer
			const buf = data as Buffer;
			// Check if it's a SharedArrayBuffer
			if (buf.buffer instanceof SharedArrayBuffer) {
				// Copy SharedArrayBuffer to regular ArrayBuffer
				const bytes = new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
				messageData = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as unknown as ArrayBuffer;
			} else {
				messageData = buf.buffer.slice(
					buf.byteOffset,
					buf.byteOffset + buf.byteLength
				) as ArrayBuffer;
			}
		} else {
			throw new Error("Invalid data type");
		}

		// Send through tunnel
		this.#sendCallback(messageData, isBinary);
	}

	close(code?: number, reason?: string): void {
		if (
			this.#readyState === 2 || // CLOSING
			this.#readyState === 3    // CLOSED
		) {
			return;
		}

		this.#readyState = 2; // CLOSING

		// Send close through tunnel
		this.#closeCallback(code, reason);

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

	addEventListener(
		type: string,
		listener: (event: any) => void,
		options?: boolean | any
	): void {
		if (typeof listener === "function") {
			let listeners = this.#eventListeners.get(type);
			if (!listeners) {
				listeners = new Set();
				this.#eventListeners.set(type, listeners);
			}
			listeners.add(listener);

			// Flush any buffered events for this type
			this.#flushBufferedEvents(type);
		}
	}

	removeEventListener(
		type: string,
		listener: (event: any) => void,
		options?: boolean | any
	): void {
		if (typeof listener === "function") {
			const listeners = this.#eventListeners.get(type);
			if (listeners) {
				listeners.delete(listener);
			}
		}
	}

	dispatchEvent(event: any): boolean {
		// Simple implementation
		return true;
	}

	#fireEvent(type: string, event: any): void {
		// Call all registered event listeners
		const listeners = this.#eventListeners.get(type);
		let hasListeners = false;

		if (listeners && listeners.size > 0) {
			hasListeners = true;
			for (const listener of listeners) {
				try {
					listener.call(this, event);
				} catch (error) {
					console.error("Error in websocket event listener", { error, type });
				}
			}
		}

		// Call the onX property if set
		switch (type) {
			case "open":
				if (this.#onopen) {
					hasListeners = true;
					try {
						this.#onopen.call(this, event);
					} catch (error) {
						console.error("Error in onopen handler", { error });
					}
				}
				break;
			case "close":
				if (this.#onclose) {
					hasListeners = true;
					try {
						this.#onclose.call(this, event);
					} catch (error) {
						console.error("Error in onclose handler", { error });
					}
				}
				break;
			case "error":
				if (this.#onerror) {
					hasListeners = true;
					try {
						this.#onerror.call(this, event);
					} catch (error) {
						console.error("Error in onerror handler", { error });
					}
				}
				break;
			case "message":
				if (this.#onmessage) {
					hasListeners = true;
					try {
						this.#onmessage.call(this, event);
					} catch (error) {
						console.error("Error in onmessage handler", { error });
					}
				}
				break;
		}

		// Buffer the event if no listeners are registered
		if (!hasListeners) {
			this.#bufferedEvents.push({ type, event });
		}
	}

	#flushBufferedEvents(type: string): void {
		const eventsToFlush = this.#bufferedEvents.filter(
			(buffered) => buffered.type === type
		);
		this.#bufferedEvents = this.#bufferedEvents.filter(
			(buffered) => buffered.type !== type
		);

		for (const { event } of eventsToFlush) {
			// Re-fire the event, which will now have listeners
			const listeners = this.#eventListeners.get(type);
			if (listeners) {
				for (const listener of listeners) {
					try {
						listener.call(this, event);
					} catch (error) {
						console.error("Error in websocket event listener", {
							error,
							type,
						});
					}
				}
			}

			// Also call the onX handler if it exists
			switch (type) {
				case "open":
					if (this.#onopen) {
						try {
							this.#onopen.call(this, event);
						} catch (error) {
							console.error("Error in onopen handler", { error });
						}
					}
					break;
				case "close":
					if (this.#onclose) {
						try {
							this.#onclose.call(this, event);
						} catch (error) {
							console.error("Error in onclose handler", { error });
						}
					}
					break;
				case "error":
					if (this.#onerror) {
						try {
							this.#onerror.call(this, event);
						} catch (error) {
							console.error("Error in onerror handler", { error });
						}
					}
					break;
				case "message":
					if (this.#onmessage) {
						try {
							this.#onmessage.call(this, event);
						} catch (error) {
							console.error("Error in onmessage handler", { error });
						}
					}
					break;
			}
		}
	}

	// Internal methods called by the Tunnel class
	_handleOpen(): void {
		if (this.#readyState !== 0) { // CONNECTING
			return;
		}

		this.#readyState = 1; // OPEN
		
		const event = {
			type: "open",
			target: this,
		};
		
		this.#fireEvent("open", event);
	}

	_handleMessage(data: string | Uint8Array, isBinary: boolean): void {
		if (this.#readyState !== 1) { // OPEN
			return;
		}

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
						data.byteOffset + data.byteLength
					);
				} else {
					messageData = data;
				}
			} else {
				// Blob type - not commonly used in Node.js
				throw new Error("Blob binaryType not supported in tunnel adapter");
			}
		} else {
			messageData = data;
		}

		const event = {
			data: messageData,
			type: "message",
			target: this,
		};

		this.#fireEvent("message", event);
	}

	_handleClose(code?: number, reason?: string): void {
		if (this.#readyState === 3) { // CLOSED
			return;
		}

		this.#readyState = 3; // CLOSED

		const event = {
			wasClean: true,
			code: code || 1000,
			reason: reason || "",
			type: "close",
			target: this,
		};

		this.#fireEvent("close", event);
	}

	_handleError(error: Error): void {
		const event = {
			type: "error",
			target: this,
			error,
		};

		this.#fireEvent("error", event);
	}

	// WebSocket constants for compatibility
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	// Instance constants
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