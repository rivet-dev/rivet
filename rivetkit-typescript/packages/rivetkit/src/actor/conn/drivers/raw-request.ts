import type { ConnDriver } from "../driver";
import { DriverReadyState } from "../driver";

/**
 * Creates a raw HTTP connection driver.
 *
 * This driver is used for raw HTTP connections that don't use the RivetKit protocol.
 * Unlike the standard HTTP driver, this provides connection lifecycle management
 * for tracking the HTTP request through the actor's onRequest handler.
 */
export function createRawRequestDriver(): ConnDriver {
	return {
		type: "raw-request",

		disconnect: async () => {
			// Noop
		},

		getConnectionReadyState: (): DriverReadyState | undefined => {
			// HTTP connections are always considered open until the request completes
			return DriverReadyState.OPEN;
		},
	};
}
