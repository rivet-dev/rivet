import { actor } from "rivetkit";

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount: number) => (c.state.count += amount),
		getCount: (c) => c.state.count,
	},
});
