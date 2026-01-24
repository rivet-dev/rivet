import { logger } from "./log";

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

		// Check for native WebSocket in multiple ways to handle different runtimes
		// Some runtimes expose WebSocket on globalThis but not as a global variable
		const nativeWebSocket =
			typeof WebSocket !== "undefined"
				? WebSocket
				: typeof globalThis !== "undefined" && globalThis.WebSocket
					? globalThis.WebSocket
					: undefined;

		if (nativeWebSocket) {
			// Native WebSocket (browsers, Deno, Node 22+, edge runtimes like Convex/Cloudflare)
			_WebSocket = nativeWebSocket as unknown as typeof WebSocket;
			logger()?.debug({ msg: "using native websocket" });
		} else {
			// Node.js package - only for older Node.js without native WebSocket
			try {
				// Use new Function to completely hide the import from bundlers.
				// Bundlers like esbuild statically analyze imports and include
				// dependencies even with variable indirection. This technique
				// prevents any bundler from seeing the "ws" string.
				// Edge runtimes should hit the native WebSocket branch above.
				const dynamicImport = new Function(
					"moduleName",
					"return import(moduleName)",
				) as (moduleName: string) => Promise<any>;
				const ws = await dynamicImport("ws");
				_WebSocket = ws.default as unknown as typeof WebSocket;
				logger()?.debug({ msg: "using websocket from npm" });
			} catch {
				// WS not available
				_WebSocket = class MockWebSocket {
					constructor() {
						throw new Error(
							'WebSocket support requires installing the "ws" peer dependency.',
						);
					}
				} as unknown as typeof WebSocket;
				logger()?.debug({ msg: "using mock websocket" });
			}
		}

		return _WebSocket;
	})();

	return webSocketPromise;
}
