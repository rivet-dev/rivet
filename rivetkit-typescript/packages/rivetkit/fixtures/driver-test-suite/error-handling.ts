import { actor, UserError } from "rivetkit";

export const errorHandlingActor = actor({
	state: {
		errorLog: [] as string[],
	},
	actions: {
		// Action that throws a UserError with just a message
		throwSimpleError: () => {
			throw new UserError("Simple error message");
		},

		// Action that throws a UserError with code and metadata
		throwDetailedError: () => {
			throw new UserError("Detailed error message", {
				code: "detailed_error",
				metadata: {
					reason: "test",
					timestamp: Date.now(),
				},
			});
		},

		// Action that throws an internal error
		throwInternalError: () => {
			throw new Error("This is an internal error");
		},

		// Action that returns successfully
		successfulAction: () => {
			return "success";
		},

		// Action that times out (simulated with a long delay)
		timeoutAction: async (c) => {
			// This action should time out if the timeout is configured
			return new Promise((resolve) => {
				setTimeout(() => {
					resolve("This should not be reached if timeout works");
				}, 10000); // 10 seconds
			});
		},

		// Action with configurable delay to test timeout edge cases
		delayedAction: async (c, delayMs: number) => {
			return new Promise((resolve) => {
				setTimeout(() => {
					resolve(`Completed after ${delayMs}ms`);
				}, delayMs);
			});
		},

		// Log an error for inspection
		logError: (c, error: string) => {
			c.state.errorLog.push(error);
			return c.state.errorLog;
		},

		// Get the error log
		getErrorLog: (c) => {
			return c.state.errorLog;
		},

		// Clear the error log
		clearErrorLog: (c) => {
			c.state.errorLog = [];
			return true;
		},
	},
	options: {
		actionTimeout: 500, // 500ms timeout for actions
	},
});

// Actor with custom timeout
export const customTimeoutActor = actor({
	state: {},
	actions: {
		quickAction: async () => {
			await new Promise((resolve) => setTimeout(resolve, 50));
			return "Quick action completed";
		},
		slowAction: async () => {
			await new Promise((resolve) => setTimeout(resolve, 300));
			return "Slow action completed";
		},
	},
	options: {
		actionTimeout: 200, // 200ms timeout
	},
});
