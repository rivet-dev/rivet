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
 * Callback counters and derived values live in vars so onStateChange stays read-only.
 */
export const stateChangeRecursionActor = actor({
	state: {
		value: 0,
		derivedValue: 0,
	},
	vars: {
		onStateChangeCallCount: 0,
		derivedValue: 0,
	},
	onStateChange: (c) => {
		c.vars.derivedValue = c.state.value * 2;
		c.vars.onStateChangeCallCount++;
	},
	actions: {
		setValue: (c, newValue: number) => {
			c.state.value = newValue;
			return c.state.value;
		},
		getDerivedValue: (c) => {
			return c.vars.derivedValue;
		},
		getOnStateChangeCallCount: (c) => {
			return c.vars.onStateChangeCallCount;
		},
		getAll: (c) => {
			return {
				value: c.state.value,
				derivedValue: c.vars.derivedValue,
				callCount: c.vars.onStateChangeCallCount,
			};
		},
	},
});

export const stateChangeReentrantMutationActor = actor({
	state: {
		value: 0,
		derivedValue: 0,
	},
	vars: {
		callCount: 0,
		errorGroup: "",
		errorCode: "",
	},
	onStateChange: (c) => {
		c.vars.callCount++;

		try {
			const state = c.state as { value: number; derivedValue: number };
			// Deliberately exercise re-entrant state mutation rejection.
			state.derivedValue = state.value * 2;
		} catch (error) {
			c.vars.errorGroup = (error as { group?: string }).group ?? "";
			c.vars.errorCode = (error as { code?: string }).code ?? "";
		}
	},
	actions: {
		setValue: (c, newValue: number) => {
			c.state.value = newValue;
			return c.state.value;
		},
		getResult: (c) => {
			return {
				value: c.state.value,
				derivedValue: c.state.derivedValue,
				callCount: c.vars.callCount,
				errorGroup: c.vars.errorGroup,
				errorCode: c.vars.errorCode,
			};
		},
	},
});
