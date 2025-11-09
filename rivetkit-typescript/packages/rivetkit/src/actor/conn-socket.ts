import type { ConnDriverState } from "./conn-drivers";

export interface ConnSocket {
	/** This is the request ID provided by the given framework. If not provided this is a random UUID. */
	requestId: string;
	requestIdBuf?: ArrayBuffer;
	hibernatable: boolean;
	driverState: ConnDriverState;
}
