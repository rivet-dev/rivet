import { actor } from "rivetkit";

export const throwOnWakeActor = actor({
	state: {
		wakeAttempts: 0,
	},
	onWake: (c) => {
		c.state.wakeAttempts += 1;
		throw new Error("throw_on_wake_actor.start_failed");
	},
	actions: {
		ping: () => "pong",
	},
});
