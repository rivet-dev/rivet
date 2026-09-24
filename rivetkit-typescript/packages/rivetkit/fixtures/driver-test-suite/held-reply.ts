import { actor, event, queue } from "rivetkit";

export const heldReplyActor = actor({
	state: { count: 0 },
	events: {
		incremented: event<number>(),
	},
	queues: {
		release: queue<boolean>(),
	},
	actions: {
		getCount: (c) => c.state.count,
		/** Increments, then holds the reply until a `release` message arrives. */
		incrementAndHold: async (c) => {
			c.state.count += 1;
			c.broadcast("incremented", c.state.count);
			await c.queue.next({ names: ["release"], timeout: 10_000 });
			return c.state.count;
		},
	},
});
