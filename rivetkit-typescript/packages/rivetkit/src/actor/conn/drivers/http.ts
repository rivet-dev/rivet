import { type ConnDriver, DriverReadyState } from "../driver";

export type ConnHttpState = Record<never, never>;

export function createHttpDriver(): ConnDriver {
	return {
		type: "http",
		getConnectionReadyState(_actor, _conn) {
			// TODO: This might not be the correct logic
			return DriverReadyState.OPEN;
		},
		disconnect: async () => {
			// Noop
			// TODO: Configure with abort signals to abort the request
		},
	};
}
