import type { EventSource } from "eventsource";

/**
 * EventSource getter that returns the global EventSource.
 *
 * In Node.js environments, the node-entry.ts entrypoint injects the 'eventsource'
 * package into globalThis.EventSource before this is called.
 *
 * IMPORTANT: We use the 'eventsource' npm package instead of the browser's native
 * EventSource because we need to attach custom headers to requests.
 */
export function getEventSource(): typeof EventSource {
	if (typeof globalThis.EventSource === "undefined") {
		throw new Error(
			'EventSource is not available. In Node.js, ensure you are importing from "rivetkit" ' +
				'(not "rivetkit/browser") which sets up the EventSource polyfill.',
		);
	}
	return globalThis.EventSource as typeof EventSource;
}

/**
 * @deprecated Use getEventSource() instead. This async version exists for backwards compatibility.
 */
export async function importEventSource(): Promise<typeof EventSource> {
	return getEventSource();
}
