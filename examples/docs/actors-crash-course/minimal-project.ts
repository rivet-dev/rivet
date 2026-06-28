import { actor, event, setup } from "rivetkit";

const counter = actor({
	state: { count: 0 },
	events: {
		count: event<number>(),
	},
	actions: {
		increment: (c, amount: number) => {
			c.state.count += amount;
			c.broadcast("count", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});

registry.start();
