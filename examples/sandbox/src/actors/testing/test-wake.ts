import { actor } from "rivetkit";

export const testWake = actor({
	state: {},
	actions: {
		noop: (_c) => {
			return { ok: true };
		},
		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},
	},
});
