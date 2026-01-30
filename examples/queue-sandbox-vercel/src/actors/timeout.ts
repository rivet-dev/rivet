import { actor } from "rivetkit";

export interface TimeoutResult {
	timedOut: boolean;
	message?: unknown;
	waitedMs: number;
}

export interface TimeoutState {
	lastResult: TimeoutResult | null;
	waitStartedAt: number | null;
}

export const timeout = actor({
	state: {
		lastResult: null as TimeoutResult | null,
		waitStartedAt: null as number | null,
	},
	actions: {
		async waitForMessage(c, timeoutMs: number): Promise<TimeoutResult> {
			const startedAt = Date.now();
			c.state.waitStartedAt = startedAt;
			c.broadcast("waitStarted", { startedAt, timeoutMs });

			const msg = await c.queue.next("work", { timeout: timeoutMs });

			const waitedMs = Date.now() - startedAt;
			const result: TimeoutResult = msg
				? { timedOut: false, message: msg.body, waitedMs }
				: { timedOut: true, waitedMs };

			c.state.lastResult = result;
			c.state.waitStartedAt = null;
			c.broadcast("waitCompleted", result);
			return result;
		},
		getState(c): TimeoutState {
			return {
				lastResult: c.state.lastResult,
				waitStartedAt: c.state.waitStartedAt,
			};
		},
	},
});
