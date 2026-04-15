import { actor } from "rivetkit";

export const testCounter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount: number = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => {
			return c.state.count;
		},
		reset: (c) => {
			c.state.count = 0;
			return c.state.count;
		},
	},
});
