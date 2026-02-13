import { actor } from "rivetkit";
import { ActorError } from "@/actor/errors";

// Custom error that will be thrown in createConnState
class CustomConnectionError extends ActorError {
	constructor(message: string) {
		super("connection", "custom_error", message, { public: true });
	}
}

/**
 * Actor that throws a custom error in createConnState to test error serialization
 */
export const connErrorSerializationActor = actor({
	state: {
		value: 0,
	},
	createConnState: (_c, params: { shouldThrow?: boolean }) => {
		if (params.shouldThrow) {
			throw new CustomConnectionError("Test error from createConnState");
		}
		return { initialized: true };
	},
	actions: {
		getValue: (c) => c.state.value,
	},
});
