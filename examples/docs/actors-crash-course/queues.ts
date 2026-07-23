import { actor, queue } from "rivetkit";

const counter = actor({
	state: { value: 0 },
	queues: {
		increment: queue<{ amount: number }>({
			onMessage: async (c, message) => {
			c.state.value += message.body.amount;
			},
		}),
	},
});
