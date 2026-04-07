import { actor, UserError } from "rivetkit";

export const ON_BEFORE_CONNECT_DELAY = 200;

/**
 * Actor that delays in onBeforeConnect to test timeout behavior.
 */
export const beforeConnectTimeoutActor = actor({
	options: {
		onBeforeConnectTimeout: 100,
	},
	onBeforeConnect: async (_c, _params: {}) => {
		// Delay longer than the configured timeout
		await new Promise((resolve) =>
			setTimeout(resolve, ON_BEFORE_CONNECT_DELAY),
		);
	},
	actions: {
		ping: () => "pong",
	},
});

/**
 * Actor that throws a UserError in onBeforeConnect to test rejection.
 */
export const beforeConnectRejectActor = actor({
	onBeforeConnect: async (_c, params: { shouldReject?: boolean }) => {
		if (params?.shouldReject) {
			throw new UserError("Connection rejected by policy", {
				code: "connection_rejected",
				metadata: { reason: "test" },
			});
		}
	},
	actions: {
		ping: () => "pong",
	},
});

/**
 * Actor that throws a generic error in onBeforeConnect to test non-UserError rejection.
 */
export const beforeConnectGenericErrorActor = actor({
	onBeforeConnect: async (_c, params: { shouldFail?: boolean }) => {
		if (params?.shouldFail) {
			throw new Error("internal failure in onBeforeConnect");
		}
	},
	actions: {
		ping: () => "pong",
	},
});

/**
 * Actor that tests onStateChange recursion prevention.
 * Mutating state inside onStateChange should NOT trigger another onStateChange call.
 */
export const stateChangeRecursionActor = actor({
	state: {
		value: 0,
		derivedValue: 0,
		onStateChangeCallCount: 0,
	},
	onStateChange: (c) => {
		// This mutation should NOT trigger another onStateChange
		c.state.derivedValue = c.state.value * 2;
		c.state.onStateChangeCallCount++;
	},
	actions: {
		setValue: (c, newValue: number) => {
			c.state.value = newValue;
			return c.state.value;
		},
		getDerivedValue: (c) => {
			return c.state.derivedValue;
		},
		getOnStateChangeCallCount: (c) => {
			return c.state.onStateChangeCallCount;
		},
		getAll: (c) => {
			return {
				value: c.state.value,
				derivedValue: c.state.derivedValue,
				callCount: c.state.onStateChangeCallCount,
			};
		},
	},
});
