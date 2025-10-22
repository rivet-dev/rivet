import type { ConnDriverState } from "./conn-drivers";

export interface ConnSocket {
	socketId: string;
	driverState: ConnDriverState;
}
