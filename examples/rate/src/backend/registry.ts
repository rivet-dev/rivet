import { actor, setup } from "rivetkit";

export type RateLimitResult = {
	allowed: boolean;
	remaining: number;
	resetsIn: number;
};

export const rateLimiter = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		count: 0,
		resetAt: 0,
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		checkLimit: (c): RateLimitResult => {
			const now = Date.now();

			// Reset if expired
			if (now > c.state.resetAt) {
				// State changes are automatically persisted
				c.state.count = 0;
				c.state.resetAt = now + 60000; // 1 minute window
			}

			const allowed = c.state.count < 5;

			// Increment if allowed
			if (allowed) {
				c.state.count++;
			}

			return {
				allowed,
				remaining: Math.max(0, 5 - c.state.count),
				resetsIn: Math.max(
					0,
					Math.round((c.state.resetAt - now) / 1000),
				),
			};
		},

		getStatus: (c) => ({
			count: c.state.count,
			resetAt: c.state.resetAt,
			remaining: Math.max(0, 5 - c.state.count),
			resetsIn: Math.max(
				0,
				Math.round((c.state.resetAt - Date.now()) / 1000),
			),
		}),

		reset: (c) => {
			c.state.count = 0;
			c.state.resetAt = 0;
			return { success: true };
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { rateLimiter },
});
