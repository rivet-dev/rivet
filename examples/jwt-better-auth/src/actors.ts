import { actor, setup } from "rivetkit";

export const user = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount: number) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

export const registry = setup({ use: { user } });
