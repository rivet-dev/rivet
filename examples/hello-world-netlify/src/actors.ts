import { actor, setup } from "rivetkit";

export const counter = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		count: 0,
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		increment: (c, amount: number) => {
			c.state.count += amount;
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},

		getCount: (c) => c.state.count,
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { counter },
});