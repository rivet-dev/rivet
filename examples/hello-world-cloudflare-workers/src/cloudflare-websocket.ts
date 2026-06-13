// Cloudflare Workers do not expose the `new WebSocket()` constructor that the
// Rivet envoy client uses to reach the engine. This shim implements that
// constructor on top of the fetch-based WebSocket upgrade that Workers support.

type CloudflareWebSocket = WebSocket & { accept(): void };

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
	#socket: CloudflareWebSocket | undefined;
	#pending: Array<string | ArrayBuffer | ArrayBufferView> = [];

	constructor(url: string, protocols?: string | string[]) {
		void this.#connect(url, protocols);
	}

	async #connect(url: string, protocols?: string | string[]) {
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
				response as unknown as { webSocket: CloudflareWebSocket | null }
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
			socket.addEventListener("error", () => {
				this.onerror?.(new Event("error"));
			});
			this.onopen?.(new Event("open"));
			for (const data of this.#pending.splice(0)) {
				socket.send(data);
			}
		} catch {
			this.readyState = FetchWebSocket.CLOSED;
			this.onerror?.(new Event("error"));
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

(globalThis as unknown as { WebSocket: typeof WebSocket }).WebSocket =
	FetchWebSocket as unknown as typeof WebSocket;
