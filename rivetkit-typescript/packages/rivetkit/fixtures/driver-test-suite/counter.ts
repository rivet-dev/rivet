import { actor } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
		setCount: (c, x: number) => {
			c.state.count = x;
			c.broadcast("newCount", x);
			return c.state.count;
		},
		getCount: (c) => {
			return c.state.count;
		},
		getKey: (c) => {
			return c.key;
		},
	},
});
