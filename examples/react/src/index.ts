import { actor, setup, event } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	events: {
		newCount: event<number>(),
	},
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
	},
});

// Register actors and start the server. https://rivet.dev/docs/setup
export const registry = setup({
	use: { counter },
});

registry.start();
