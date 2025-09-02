// Global singleton promise that will be reused for subsequent calls
let webSocketPromise: Promise<typeof WebSocket> | null = null;

export async function importWebSocket(): Promise<typeof WebSocket> {
	// Return existing promise if we already started loading
	if (webSocketPromise !== null) {
		return webSocketPromise;
	}

	// Create and store the promise
	webSocketPromise = (async () => {
		let _WebSocket: typeof WebSocket;

		if (typeof WebSocket !== "undefined") {
			// Native
			_WebSocket = WebSocket as unknown as typeof WebSocket;
			console.debug("using native websocket");
		} else {
			// Node.js package
			try {
				const ws = await import("ws");
				_WebSocket = ws.default as unknown as typeof WebSocket;
				console.debug("using websocket from npm");
			} catch {
				// WS not available
				_WebSocket = class MockWebSocket {
					constructor() {
						throw new Error(
							'WebSocket support requires installing the "ws" peer dependency.',
						);
					}
				} as unknown as typeof WebSocket;
				console.debug("using mock websocket");
			}
		}

		return _WebSocket;
	})();

	return webSocketPromise;
}
