import type {
	RivetCloseEvent,
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import { getLogger } from "./log";

function logger() {
	return getLogger("virtual-websocket");
}

export interface VirtualWebSocketOptions {
	/** Get the current ready state */
	getReadyState: () => 0 | 1 | 2 | 3;
	/** Called when send() is invoked */
	onSend: (data: string | ArrayBufferLike | Blob | ArrayBufferView) => void;
	/** Called when close() is invoked */
	onClose: (code: number, reason: string) => void;
}

/**
 * Virtual WebSocket implementation that dispatches events and delegates
 * send/close to callbacks. Used to create linked WebSocket pairs.
 */
export class VirtualWebSocket implements UniversalWebSocket {
	readonly CONNECTING = 0 as const;
	readonly OPEN = 1 as const;
	readonly CLOSING = 2 as const;
	readonly CLOSED = 3 as const;

	#options: VirtualWebSocketOptions;
	#listeners: Map<string, ((ev: any) => void)[]> = new Map();
	#onopen: ((event: RivetEvent) => void) | null = null;
	#onclose: ((event: RivetCloseEvent) => void) | null = null;
	#onerror: ((event: RivetEvent) => void) | null = null;
	#onmessage: ((event: RivetMessageEvent) => void) | null = null;

	constructor(options: VirtualWebSocketOptions) {
		this.#options = options;
	}

	// UniversalWebSocket properties
	get readyState(): 0 | 1 | 2 | 3 {
		return this.#options.getReadyState();
	}

	get binaryType(): "arraybuffer" | "blob" {
		return "arraybuffer";
	}

	set binaryType(_value: "arraybuffer" | "blob") {
		// Ignored - always use arraybuffer
	}

	get bufferedAmount(): number {
		return 0;
	}

	get extensions(): string {
		return "";
	}

	get protocol(): string {
		return "";
	}

	get url(): string {
		return "";
	}

	// UniversalWebSocket methods
	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		const state = this.readyState;
		if (state === this.CONNECTING) {
			throw new DOMException(
				"WebSocket is still in CONNECTING state",
				"InvalidStateError",
			);
		}
		if (state === this.CLOSING || state === this.CLOSED) {
			logger().debug({
				msg: "ignoring send, websocket is closing or closed",
				readyState: state,
			});
			return;
		}
		this.#options.onSend(data);
	}

	close(code = 1000, reason = ""): void {
		const state = this.readyState;
		if (state === this.CLOSED || state === this.CLOSING) {
			return;
		}
		this.#options.onClose(code, reason);
	}

	addEventListener(type: string, listener: (ev: any) => void): void {
		if (!this.#listeners.has(type)) {
			this.#listeners.set(type, []);
		}
		this.#listeners.get(type)!.push(listener);
	}

	removeEventListener(type: string, listener: (ev: any) => void): void {
		const listeners = this.#listeners.get(type);
		if (listeners) {
			const index = listeners.indexOf(listener);
			if (index !== -1) {
				listeners.splice(index, 1);
			}
		}
	}

	dispatchEvent(event: RivetEvent): boolean {
		this.#dispatch(event.type, event);
		return true;
	}

	// on* property getters/setters
	get onopen(): ((event: RivetEvent) => void) | null {
		return this.#onopen;
	}
	set onopen(fn: ((event: RivetEvent) => void) | null) {
		this.#onopen = fn;
	}

	get onclose(): ((event: RivetCloseEvent) => void) | null {
		return this.#onclose;
	}
	set onclose(fn: ((event: RivetCloseEvent) => void) | null) {
		this.#onclose = fn;
	}

	get onerror(): ((event: RivetEvent) => void) | null {
		return this.#onerror;
	}
	set onerror(fn: ((event: RivetEvent) => void) | null) {
		this.#onerror = fn;
	}

	get onmessage(): ((event: RivetMessageEvent) => void) | null {
		return this.#onmessage;
	}
	set onmessage(fn: ((event: RivetMessageEvent) => void) | null) {
		this.#onmessage = fn;
	}

	// Helper methods to trigger events from the other side
	triggerOpen(): void {
		const event = {
			type: "open",
			target: this,
			currentTarget: this,
		} as unknown as RivetEvent;
		this.#dispatch("open", event);
	}

	triggerMessage(data: any): void {
		logger().debug({
			msg: "triggering message event",
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

		const event = {
			type: "message",
			data,
			target: this,
			currentTarget: this,
		} as unknown as RivetMessageEvent;
		this.#dispatch("message", event);
	}

	triggerClose(code: number, reason: string): void {
		const event = {
			type: "close",
			code,
			reason,
			wasClean: code === 1000,
			target: this,
			currentTarget: this,
		} as unknown as RivetCloseEvent;
		this.#dispatch("close", event);
	}

	triggerError(error: unknown): void {
		const event = {
			type: "error",
			target: this,
			currentTarget: this,
			error,
			message: error instanceof Error ? error.message : String(error),
		} as unknown as RivetEvent;
		this.#dispatch("error", event);
		logger().error({ msg: "websocket error", error });
	}

	#dispatch(type: string, event: any): void {
		// Dispatch to addEventListener listeners
		const listeners = this.#listeners.get(type);
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

		// Dispatch to on* property handlers
		switch (type) {
			case "open":
				if (this.#onopen) {
					try {
						this.#onopen(event);
					} catch (err) {
						logger().error({ msg: "error in onopen handler", error: err });
					}
				}
				break;
			case "close":
				if (this.#onclose) {
					try {
						this.#onclose(event);
					} catch (err) {
						logger().error({ msg: "error in onclose handler", error: err });
					}
				}
				break;
			case "error":
				if (this.#onerror) {
					try {
						this.#onerror(event);
					} catch (err) {
						logger().error({ msg: "error in onerror handler", error: err });
					}
				}
				break;
			case "message":
				if (this.#onmessage) {
					try {
						this.#onmessage(event);
					} catch (err) {
						logger().error({ msg: "error in onmessage handler", error: err });
					}
				}
				break;
		}
	}
}
