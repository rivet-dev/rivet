// Cloudflare Workers do not implement an outbound `new WebSocket(url)` client
// constructor, but the wasm actor runtime opens its tunnel to the Rivet engine
// through `globalThis.WebSocket`. This shim translates that constructor into
// Cloudflare's fetch-based upgrade (`fetch(url, { Upgrade })` + `response.webSocket`)
// and installs itself on `globalThis` so both the wasm tunnel and the TypeScript
// client path resolve a working implementation.

type WebSocketProtocolInput = string | string[] | undefined;

type CloudflareSocket = WebSocket & { accept(): void };

class FetchWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	binaryType: BinaryType = "arraybuffer";
	onopen: ((event: Event) => void) | null = null;
	onmessage: ((event: MessageEvent) => void) | null = null;
	onclose: ((event: CloseEvent) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;
	readyState = FetchWebSocket.CONNECTING;
	#socket: CloudflareSocket | undefined;
	#pending: Array<string | ArrayBuffer | ArrayBufferView> = [];

	constructor(url: string, protocols?: WebSocketProtocolInput) {
		void this.#connect(url, protocols);
	}

	async #connect(url: string, protocols?: WebSocketProtocolInput) {
		try {
			const protocolList = Array.isArray(protocols)
				? protocols
				: protocols
					? [protocols]
					: [];
			const headers = new Headers({ Upgrade: "websocket" });
			if (protocolList.length > 0) {
				headers.set("Sec-WebSocket-Protocol", protocolList.join(", "));
			}
			const response = await fetch(
				url.replace(/^ws:/, "http:").replace(/^wss:/, "https:"),
				{ headers },
			);
			const socket = (
				response as unknown as { webSocket: CloudflareSocket | null }
			).webSocket;
			if (!socket) {
				throw new Error(
					`websocket upgrade failed with status ${response.status}`,
				);
			}

			socket.accept();
			socket.binaryType = this.binaryType;
			this.#socket = socket;
			this.readyState = FetchWebSocket.OPEN;
			socket.addEventListener("message", (event) => {
				this.onmessage?.(event);
			});
			socket.addEventListener("close", (event) => {
				this.readyState = FetchWebSocket.CLOSED;
				this.onclose?.(event);
			});
			socket.addEventListener("error", (event) => {
				this.onerror?.(event);
			});
			this.onopen?.(new Event("open"));
			for (const data of this.#pending.splice(0)) {
				socket.send(data);
			}
		} catch (error) {
			console.error("rivetkit cloudflare websocket shim failed", error);
			this.readyState = FetchWebSocket.CLOSED;
			this.onerror?.(error instanceof Event ? error : new Event("error"));
			this.onclose?.(new CloseEvent("close", { code: 1006 }));
		}
	}

	send(data: string | ArrayBuffer | ArrayBufferView) {
		if (this.readyState === FetchWebSocket.CONNECTING) {
			this.#pending.push(data);
			return;
		}
		this.#socket?.send(data);
	}

	close(code?: number, reason?: string) {
		this.readyState = FetchWebSocket.CLOSING;
		this.#socket?.close(code, reason);
	}
}

const globalScope = globalThis as unknown as {
	WebSocket: typeof WebSocket;
	__RIVETKIT_CF_WEBSOCKET_INSTALLED__?: boolean;
};

// Install once per isolate.
if (!globalScope.__RIVETKIT_CF_WEBSOCKET_INSTALLED__) {
	globalScope.WebSocket = FetchWebSocket as unknown as typeof WebSocket;
	globalScope.__RIVETKIT_CF_WEBSOCKET_INSTALLED__ = true;
}

export { FetchWebSocket };
