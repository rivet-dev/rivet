import { type ActorContextOf, actor, event } from "rivetkit";

const ticker = actor({
	events: {
		tick: event<number>(),
	},
	vars: {
		ticks: 0,
		// `onWake` stores its loop here so `onSleep` can wait for it.
		loop: null as Promise<void> | null,
	},

	// `onWake` runs on every start, so the loop restarts after a sleep or crash
	onWake: (c) => {
		c.vars.loop = tickLoop(c).catch((error) => {
			c.log.error({ msg: "tick loop failed", error });
		});
	},

	// `c.abortSignal` fires before `onSleep`, so the loop is already exiting
	onSleep: async (c) => {
		await c.vars.loop;
	},

	actions: {
		getTicks: (c) => c.vars.ticks,
	},
});

async function tickLoop(c: ActorContextOf<typeof ticker>): Promise<void> {
	while (!c.aborted) {
		c.vars.ticks += 1;
		c.broadcast("tick", c.vars.ticks);
		await delay(1_000, c.abortSignal);
	}
}

// Resolve after `ms`, or as soon as the actor starts shutting down
function delay(ms: number, signal: AbortSignal): Promise<void> {
	return new Promise((resolve) => {
		if (signal.aborted) {
			resolve();
			return;
		}

		const finish = () => {
			clearTimeout(timer);
			signal.removeEventListener("abort", finish);
			resolve();
		};
		const timer = setTimeout(finish, ms);
		signal.addEventListener("abort", finish, { once: true });
	});
}
