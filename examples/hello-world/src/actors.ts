import { actor, setup } from "rivetkit";

// Event payload type for count changes
export type CountChangedEvent = {
	count: number;
	updatedAt: number;
};

export const counter = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		count: 0,
		lastUpdatedAt: 0,
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		increment: (c, amount: number) => {
			c.state.count += amount;
			c.state.lastUpdatedAt = Date.now();
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("countChanged", {
				count: c.state.count,
				updatedAt: c.state.lastUpdatedAt,
			} satisfies CountChangedEvent);
			return c.state;
		},

		decrement: (c, amount: number) => {
			c.state.count -= amount;
			c.state.lastUpdatedAt = Date.now();
			c.broadcast("countChanged", {
				count: c.state.count,
				updatedAt: c.state.lastUpdatedAt,
			} satisfies CountChangedEvent);
			return c.state;
		},

		reset: (c) => {
			c.state.count = 0;
			c.state.lastUpdatedAt = Date.now();
			c.broadcast("countChanged", {
				count: c.state.count,
				updatedAt: c.state.lastUpdatedAt,
			} satisfies CountChangedEvent);
			return c.state;
		},

		getState: (c) => c.state,
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { counter },
});
