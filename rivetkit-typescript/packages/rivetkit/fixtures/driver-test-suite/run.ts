import { actor } from "rivetkit";
import type { registry } from "./registry";

export const RUN_SLEEP_TIMEOUT = 1000;

// Actor that tracks tick counts and respects abort signal
export const runWithTicks = actor({
	state: {
		tickCount: 0,
		lastTickAt: 0,
		runStarted: false,
		runExited: false,
	},
	run: async (c) => {
		c.state.runStarted = true;
		c.log.info("run handler started");

		while (!c.aborted) {
			c.state.tickCount += 1;
			c.state.lastTickAt = Date.now();
			c.log.info({ msg: "tick", tickCount: c.state.tickCount });

			// Wait 50ms between ticks, or exit early if aborted
			await new Promise<void>((resolve) => {
				const timeout = setTimeout(resolve, 50);
				c.abortSignal.addEventListener(
					"abort",
					() => {
						clearTimeout(timeout);
						resolve();
					},
					{ once: true },
				);
			});
		}

		c.state.runExited = true;
		c.log.info("run handler exiting gracefully");
	},
	actions: {
		getState: (c) => ({
			tickCount: c.state.tickCount,
			lastTickAt: c.state.lastTickAt,
			runStarted: c.state.runStarted,
			runExited: c.state.runExited,
		}),
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
		runStopTimeout: 1000,
	},
});

// Actor that consumes from a queue in the run handler
export const runWithQueueConsumer = actor({
	state: {
		messagesReceived: [] as Array<{ name: string; body: unknown }>,
		runStarted: false,
		wakeCount: 0,
	},
	onWake: (c) => {
		c.state.wakeCount += 1;
	},
	run: async (c) => {
		c.state.runStarted = true;
		c.log.info("run handler started, waiting for messages");

		while (!c.aborted) {
			const message = await c.queue.next({ names: ["messages"] });
			if (message) {
				c.log.info({ msg: "received message", body: message.body });
				c.state.messagesReceived.push({
					name: message.name,
					body: message.body,
				});
			}
		}

		c.log.info("run handler exiting gracefully");
	},
	actions: {
		getState: (c) => ({
			messagesReceived: c.state.messagesReceived,
			runStarted: c.state.runStarted,
			wakeCount: c.state.wakeCount,
		}),
		sendMessage: async (c, body: unknown) => {
			const client = c.client<typeof registry>();
			const handle = client.runWithQueueConsumer.getForId(c.actorId);
			await handle.send("messages", body);
			return true;
		},
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
		runStopTimeout: 1000,
	},
});

// Actor that exits the run handler after a short delay to test crash behavior
export const runWithEarlyExit = actor({
	state: {
		runStarted: false,
		destroyCalled: false,
	},
	run: async (c) => {
		c.state.runStarted = true;
		c.log.info("run handler started, will exit after delay");
		// Wait a bit so we can observe the runStarted state before exit
		await new Promise((resolve) => setTimeout(resolve, 200));
		c.log.info("run handler exiting early");
		// Exit without respecting abort signal
	},
	onDestroy: (c) => {
		c.state.destroyCalled = true;
	},
	actions: {
		getState: (c) => ({
			runStarted: c.state.runStarted,
			destroyCalled: c.state.destroyCalled,
		}),
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});

// Actor that throws an error in the run handler to test crash behavior
export const runWithError = actor({
	state: {
		runStarted: false,
		destroyCalled: false,
	},
	run: async (c) => {
		c.state.runStarted = true;
		c.log.info("run handler started, will throw error");
		await new Promise((resolve) => setTimeout(resolve, 200));
		throw new Error("intentional error in run handler");
	},
	onDestroy: (c) => {
		c.state.destroyCalled = true;
	},
	actions: {
		getState: (c) => ({
			runStarted: c.state.runStarted,
			destroyCalled: c.state.destroyCalled,
		}),
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});

// Actor without a run handler for comparison
export const runWithoutHandler = actor({
	state: {
		wakeCount: 0,
	},
	onWake: (c) => {
		c.state.wakeCount += 1;
	},
	actions: {
		getState: (c) => ({
			wakeCount: c.state.wakeCount,
		}),
	},
	options: {
		sleepTimeout: RUN_SLEEP_TIMEOUT,
	},
});
