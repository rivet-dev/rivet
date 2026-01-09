import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});
