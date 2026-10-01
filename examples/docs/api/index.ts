import { actor, event, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	events: {
		countChanged: event<number>(),
	},
	onRequest: (c, request) => {
		if (request.method === "POST") c.state.count++;
		return Response.json({ count: c.state.count });
	},
	actions: {
		increment: (c, amount: number) => {
			c.state.count += amount;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({ use: { counter } });
registry.start();
