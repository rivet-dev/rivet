/**
 * WebSocket getter that returns the global WebSocket.
 *
 * In Node.js environments, the node-entry.ts entrypoint injects the 'ws'
 * package into globalThis.WebSocket before this is called.
 *
 * In browser/edge environments, WebSocket is natively available on globalThis.
 */
export function getWebSocket(): typeof WebSocket {
	if (typeof globalThis.WebSocket === "undefined") {
		throw new Error(
			'WebSocket is not available. In Node.js, ensure you are importing from "rivetkit" ' +
				'(not "rivetkit/browser") which sets up the WebSocket polyfill.',
		);
	}
	return globalThis.WebSocket;
}

/**
 * @deprecated Use getWebSocket() instead. This async version exists for backwards compatibility.
 */
export async function importWebSocket(): Promise<typeof WebSocket> {
	return getWebSocket();
}
