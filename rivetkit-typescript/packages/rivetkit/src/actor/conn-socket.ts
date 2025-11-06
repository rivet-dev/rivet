import type { ConnDriverState } from "./conn-drivers";

export interface ConnSocket {
	requestId: string;
	requestIdBuf?: ArrayBuffer;
	hibernatable: boolean;
	driverState: ConnDriverState;
}
