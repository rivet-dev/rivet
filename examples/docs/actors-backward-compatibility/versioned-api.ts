import { actor, event, setup } from "rivetkit";
import { z } from "zod";

export const counter = actor({
	state: { value: 0 },
	queues: {
		"v1.add": { message: z.object({ amount: z.number() }) },
		"v2.add": { message: z.object({ delta: z.number() }) },
	},
	events: {
		"v1.changed": event<{ count: number }>(),
		"v2.changed": event<{ value: number }>(),
	},
	actions: {
		v1: { get: (c) => ({ count: c.state.value }) },
		v2: { get: (c) => ({ value: c.state.value }) },
	},
	run: async (c) => {
		for await (const message of c.queue.iter()) {
			if (message.name === "v1.add") {
				c.state.value += message.body.amount;
			} else if (message.name === "v2.add") {
				c.state.value += message.body.delta;
			}

			c.broadcast("v1.changed", { count: c.state.value });
			c.broadcast("v2.changed", { value: c.state.value });
		}
	},
});

export const registry = setup({ use: { counter } });
