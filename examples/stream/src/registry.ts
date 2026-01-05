import { actor, setup } from "rivetkit";

export type StreamState = {
	topValues: number[];
};

const streamProcessor = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		topValues: [] as number[],
		totalValues: 0,
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getTopValues: (c) => c.state.topValues,

		getStats: (c) => ({
			topValues: c.state.topValues,
			totalCount: c.state.totalValues,
			highestValue:
				c.state.topValues.length > 0 ? c.state.topValues[0] : null,
		}),

		addValue: (c, value: number) => {
			// State changes are automatically persisted
			c.state.totalValues++;

			// Insert new value if needed
			const insertAt = c.state.topValues.findIndex((v) => value > v);
			if (insertAt === -1 && c.state.topValues.length < 3) {
				// Add to end if not better than existing values but we have space
				c.state.topValues.push(value);
			} else if (insertAt !== -1) {
				// Insert at the correct position
				c.state.topValues.splice(insertAt, 0, value);
			}

			// Keep only top 3
			if (c.state.topValues.length > 3) {
				c.state.topValues.length = 3;
			}

			// Sort descending to ensure correct order
			c.state.topValues.sort((a, b) => b - a);

			const result = {
				topValues: c.state.topValues,
				totalCount: c.state.totalValues,
				highestValue:
					c.state.topValues.length > 0 ? c.state.topValues[0] : null,
			};

			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("updated", result);

			return c.state.topValues;
		},

		reset: (c) => {
			c.state.topValues = [];
			c.state.totalValues = 0;

			const result = {
				topValues: c.state.topValues,
				totalCount: c.state.totalValues,
				highestValue: null,
			};

			c.broadcast("updated", result);

			return result;
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { streamProcessor },
});
