import type { EventSource } from "eventsource";
import { logger } from "@/client/log";

// Global singleton promise that will be reused for subsequent calls
let eventSourcePromise: Promise<typeof EventSource> | null = null;

export async function importEventSource(): Promise<typeof EventSource> {
	// IMPORTANT: Import `eventsource` from the custom `eventsource` library. We need a custom implementation
	// since we need to attach our own custom headers to the request.
	//
	// We can't use the browser-provided EventSource since it does not allow providing custom headers.

	// Return existing promise if we already started loading
	if (eventSourcePromise !== null) {
		return eventSourcePromise;
	}

	// Create and store the promise
	eventSourcePromise = (async () => {
		let _EventSource: typeof EventSource;

		// Node.js environment
		try {
			const moduleName = "eventsource";
			const es = await import(/* webpackIgnore: true */ moduleName);
			_EventSource = es.EventSource;
			logger().debug("using eventsource from npm");
		} catch (err) {
			// EventSource not available
			_EventSource = class MockEventSource {
				constructor() {
					throw new Error(
						'EventSource support requires installing the "eventsource" peer dependency.',
					);
				}
			} as unknown as typeof EventSource;
			logger().debug("using mock eventsource");
		}

		return _EventSource;
	})();

	return eventSourcePromise;
}
