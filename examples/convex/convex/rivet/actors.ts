import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

// RivetKit auto-detects the engine driver from RIVET_ENDPOINT env var
export const registry = setup({
	use: { counter },
});
