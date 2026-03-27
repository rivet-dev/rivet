import type {
	RivetCloseEvent,
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";

type WebSocketListener = (event: any) => void | Promise<void>;

interface TrackedWebSocketOptions {
	onPromise: (eventType: string, promise: Promise<void>) => void;
	onError: (eventType: string, error: unknown) => void;
}

/**
 * Wraps an actor-facing WebSocket so async event handlers can be tracked by
 * the actor lifecycle without changing the underlying socket dispatch model.
 */
export class TrackedWebSocket implements UniversalWebSocket {
	#inner: UniversalWebSocket;
	#options: TrackedWebSocketOptions;
	#listeners = new Map<string, WebSocketListener[]>();
	#onopen: ((event: RivetEvent) => void | Promise<void>) | null = null;
	#onclose: ((event: RivetCloseEvent) => void | Promise<void>) | null = null;
	#onerror: ((event: RivetEvent) => void | Promise<void>) | null = null;
	#onmessage: ((event: RivetMessageEvent) => void | Promise<void>) | null =
		null;

	constructor(inner: UniversalWebSocket, options: TrackedWebSocketOptions) {
		this.#inner = inner;
		this.#options = options;

		inner.addEventListener("open", (event) => {
			this.#dispatch("open", this.#createEvent("open", event));
		});
		inner.addEventListener("message", (event) => {
			this.#dispatch("message", this.#createEvent("message", event));
		});
		inner.addEventListener("close", (event) => {
			this.#dispatch("close", this.#createEvent("close", event));
		});
		inner.addEventListener("error", (event) => {
			this.#dispatch("error", this.#createEvent("error", event));
		});
	}

	get CONNECTING(): 0 {
		return this.#inner.CONNECTING;
	}

	get OPEN(): 1 {
		return this.#inner.OPEN;
	}

	get CLOSING(): 2 {
		return this.#inner.CLOSING;
	}

	get CLOSED(): 3 {
		return this.#inner.CLOSED;
	}

	get readyState(): 0 | 1 | 2 | 3 {
		return this.#inner.readyState;
	}

	get binaryType(): "arraybuffer" | "blob" {
		return this.#inner.binaryType;
	}

	set binaryType(value: "arraybuffer" | "blob") {
		this.#inner.binaryType = value;
	}

	get bufferedAmount(): number {
		return this.#inner.bufferedAmount;
	}

	get extensions(): string {
		return this.#inner.extensions;
	}

	get protocol(): string {
		return this.#inner.protocol;
	}

	get url(): string {
		return this.#inner.url;
	}

	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		this.#inner.send(data);
	}

	close(code?: number, reason?: string): void {
		this.#inner.close(code, reason);
	}

	addEventListener(type: string, listener: WebSocketListener): void {
		if (!this.#listeners.has(type)) {
			this.#listeners.set(type, []);
		}

		this.#listeners.get(type)!.push(listener);
	}

	removeEventListener(type: string, listener: WebSocketListener): void {
		const listeners = this.#listeners.get(type);
		if (!listeners) return;

		const index = listeners.indexOf(listener);
		if (index !== -1) {
			listeners.splice(index, 1);
		}
	}

	dispatchEvent(event: RivetEvent): boolean {
		this.#dispatch(event.type, this.#createEvent(event.type, event));
		return true;
	}

	get onopen(): ((event: RivetEvent) => void | Promise<void>) | null {
		return this.#onopen;
	}

	set onopen(fn: ((event: RivetEvent) => void | Promise<void>) | null) {
		this.#onopen = fn;
	}

	get onclose(): ((event: RivetCloseEvent) => void | Promise<void>) | null {
		return this.#onclose;
	}

	set onclose(fn:
		| ((event: RivetCloseEvent) => void | Promise<void>)
		| null,) {
		this.#onclose = fn;
	}

	get onerror(): ((event: RivetEvent) => void | Promise<void>) | null {
		return this.#onerror;
	}

	set onerror(fn: ((event: RivetEvent) => void | Promise<void>) | null) {
		this.#onerror = fn;
	}

	get onmessage():
		| ((event: RivetMessageEvent) => void | Promise<void>)
		| null {
		return this.#onmessage;
	}

	set onmessage(fn:
		| ((event: RivetMessageEvent) => void | Promise<void>)
		| null,) {
		this.#onmessage = fn;
	}

	#createEvent(type: string, event: any): any {
		switch (type) {
			case "message":
				return {
					type,
					data: event.data,
					rivetRequestId: event.rivetRequestId,
					rivetMessageIndex: event.rivetMessageIndex,
					target: this,
					currentTarget: this,
				} satisfies RivetMessageEvent;
			case "close":
				return {
					type,
					code: event.code,
					reason: event.reason,
					wasClean: event.wasClean,
					rivetRequestId: event.rivetRequestId,
					target: this,
					currentTarget: this,
				} satisfies RivetCloseEvent;
			default:
				return {
					type,
					rivetRequestId: event.rivetRequestId,
					target: this,
					currentTarget: this,
					...(event.message !== undefined
						? { message: event.message }
						: {}),
					...(event.error !== undefined
						? { error: event.error }
						: {}),
				} satisfies RivetEvent;
		}
	}

	#dispatch(type: string, event: any): void {
		const listeners = this.#listeners.get(type);
		if (listeners && listeners.length > 0) {
			for (const listener of [...listeners]) {
				this.#callHandler(type, listener, event);
			}
		}

		switch (type) {
			case "open":
				if (this.#onopen) this.#callHandler(type, this.#onopen, event);
				break;
			case "close":
				if (this.#onclose)
					this.#callHandler(type, this.#onclose, event);
				break;
			case "error":
				if (this.#onerror)
					this.#callHandler(type, this.#onerror, event);
				break;
			case "message":
				if (this.#onmessage)
					this.#callHandler(type, this.#onmessage, event);
				break;
		}
	}

	#callHandler(type: string, handler: WebSocketListener, event: any): void {
		try {
			const result = handler(event);
			if (this.#isPromiseLike(result)) {
				this.#options.onPromise(type, Promise.resolve(result));
			}
		} catch (error) {
			this.#options.onError(type, error);
		}
	}

	#isPromiseLike(value: unknown): value is PromiseLike<void> {
		return (
			typeof value === "object" &&
			value !== null &&
			"then" in value &&
			typeof value.then === "function"
		);
	}
}
