import { actor, setup, event } from "rivetkit";

export type StreamUpdate = {
	topValues: number[];
	totalCount: number;
	highestValue: number | null;
};

const streamProcessor = actor({
	state: {
		topValues: [] as number[],
		totalValues: 0,
	},
	events: {
		updated: event<StreamUpdate>(),
	},

	actions: {
		getTopValues: (c) => c.state.topValues,

		getStats: (c) => ({
			topValues: c.state.topValues,
			totalCount: c.state.totalValues,
			highestValue:
				c.state.topValues.length > 0 ? c.state.topValues[0] : null,
		}),

		addValue: (c, value: number) => {
			c.state.totalValues++;

			const insertAt = c.state.topValues.findIndex((v) => value > v);
			if (insertAt === -1 && c.state.topValues.length < 3) {
				c.state.topValues.push(value);
			} else if (insertAt !== -1) {
				c.state.topValues.splice(insertAt, 0, value);
			}

			if (c.state.topValues.length > 3) {
				c.state.topValues.length = 3;
			}

			c.state.topValues.sort((a, b) => b - a);

			const result: StreamUpdate = {
				topValues: c.state.topValues,
				totalCount: c.state.totalValues,
				highestValue:
					c.state.topValues.length > 0 ? c.state.topValues[0] : null,
			};

			c.broadcast("updated", result);
			return c.state.topValues;
		},

		reset: (c) => {
			c.state.topValues = [];
			c.state.totalValues = 0;

			const result: StreamUpdate = {
				topValues: [],
				totalCount: 0,
				highestValue: null,
			};

			c.broadcast("updated", result);
			return result;
		},
	},
});

export const registry = setup({
	use: { streamProcessor },
});
