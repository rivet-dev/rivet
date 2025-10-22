import { logger } from "@/client/log";

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
			// Browser environment
			_WebSocket = WebSocket as unknown as typeof WebSocket;
		} else {
			// Node.js environment
			try {
				const moduleName = "ws";
				const ws = await import(/* webpackIgnore: true */ moduleName);
				_WebSocket = ws.default as unknown as typeof WebSocket;
				logger().debug("using websocket from npm");
			} catch {
				// WS not available
				_WebSocket = class MockWebSocket {
					constructor() {
						throw new Error(
							'WebSocket support requires installing the "ws" peer dependency.',
						);
					}
				} as unknown as typeof WebSocket;
				logger().debug("using mock websocket");
			}
		}

		return _WebSocket;
	})();

	return webSocketPromise;
}
