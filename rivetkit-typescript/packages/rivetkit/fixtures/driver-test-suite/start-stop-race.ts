import { actor } from "rivetkit";
import type { registry } from "./registry-static";

/**
 * Actor designed to test start/stop race conditions.
 * Has a slow initialization to make race conditions easier to trigger.
 */
export const startStopRaceActor = actor({
	state: {
		initialized: false,
		startTime: 0,
		destroyCalled: false,
		startCompleted: false,
	},
	onWake: async (c) => {
		c.state.startTime = Date.now();

		// Simulate slow initialization to create window for race condition
		await new Promise((resolve) => setTimeout(resolve, 100));

		c.state.initialized = true;
		c.state.startCompleted = true;

		const client = c.client<typeof registry>();
		const observer = client.lifecycleObserver.getOrCreate(["observer"]);
		await observer.recordEvent({
			actorKey: c.key.join("/"),
			event: "started",
		});
	},
	onDestroy: async (c) => {
		c.state.destroyCalled = true;

		const client = c.client<typeof registry>();
		const observer = client.lifecycleObserver.getOrCreate(["observer"]);
		await observer.recordEvent({
			actorKey: c.key.join("/"),
			event: "destroy",
		});
	},
	actions: {
		getState: (c) => {
			return {
				initialized: c.state.initialized,
				startTime: c.state.startTime,
				destroyCalled: c.state.destroyCalled,
				startCompleted: c.state.startCompleted,
			};
		},
		ping: (c) => {
			return "pong";
		},
		destroy: (c) => {
			c.destroy();
		},
	},
});

/**
 * Observer actor to track lifecycle events from other actors
 */
export const lifecycleObserver = actor({
	state: {
		events: [] as Array<{
			actorKey: string;
			event: string;
			timestamp: number;
		}>,
	},
	actions: {
		recordEvent: (c, params: { actorKey: string; event: string }) => {
			c.state.events.push({
				actorKey: params.actorKey,
				event: params.event,
				timestamp: Date.now(),
			});
		},
		getEvents: (c) => {
			return c.state.events;
		},
		clearEvents: (c) => {
			c.state.events = [];
		},
	},
});
