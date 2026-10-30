import { actor } from "rivetkit";

export const responseStartTimeoutActor = actor({
	state: { runs: 0 },
	createVars: () => ({ waiting: [] as Array<() => void> }),
	actions: {
		// Only the first run waits for `release`, so a duplicate run reaches the
		// caller as a result instead of a timeout.
		waitForRelease: async (c) => {
			c.state.runs += 1;
			if (c.state.runs === 1) {
				await new Promise<void>((resolve) =>
					c.vars.waiting.push(resolve),
				);
			}
			return c.state.runs;
		},
		release: (c) => {
			for (const resolve of c.vars.waiting.splice(0)) resolve();
		},
		runs: (c) => c.state.runs,
	},
});
