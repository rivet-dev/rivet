import { actor, queue, setup } from "rivetkit";

const counterWorker = actor({
	state: { value: 0 },
	queues: {
		mutate: queue<{ delta: number }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter()) {
			c.state.value += message.body.delta;
		}
	},
	actions: {
		getValue: (c) => c.state.value,
	},
});

const registry = setup({ use: { counterWorker } });
