import type { ConnDriverState } from "./conn-drivers";

export interface ConnSocket {
	requestId: string;
	driverState: ConnDriverState;
}
