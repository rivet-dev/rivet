import { actor, UserError } from "rivetkit";

// Actor with synchronous actions
export const syncActionActor = actor({
	createVars: () => ({ value: 0 }),
	actions: {
		// Simple synchronous action that returns a value directly
		increment: (c, amount = 1) => {
			c.vars.value += amount;
			return c.vars.value;
		},
		// Synchronous action that returns an object
		getInfo: (c) => {
			return {
				currentValue: c.vars.value,
				timestamp: Date.now(),
			};
		},
		// Synchronous action with no return value (void)
		reset: (c) => {
			c.vars.value = 0;
		},
	},
});

// Actor with asynchronous actions
export const asyncActionActor = actor({
	createVars: () => ({ value: 0, data: null as any }),
	actions: {
		// Async action with a delay
		delayedIncrement: async (c, amount = 1) => {
			await Promise.resolve();
			c.vars.value += amount;
			return c.vars.value;
		},
		// Async action that simulates an API call
		fetchData: async (c, id: string) => {
			await Promise.resolve();

			// Simulate response data
			const data = { id, timestamp: Date.now() };
			c.vars.data = data;
			return data;
		},
		// Async action with error handling
		asyncWithError: async (_c, shouldError: boolean) => {
			await Promise.resolve();

			if (shouldError) {
				throw new UserError("Intentional error");
			}

			return "Success";
		},
	},
});

// Actor with promise actions
export const promiseActor = actor({
	createVars: () => ({ results: [] as string[] }),
	actions: {
		// Action that returns a resolved promise
		resolvedPromise: (_c) => {
			return Promise.resolve("resolved value");
		},
		// Action that returns a promise that resolves after a delay
		delayedPromise: (c): Promise<string> => {
			return new Promise<string>((resolve) => {
				c.vars.results.push("delayed");
				resolve("delayed value");
			});
		},
		// Action that returns a rejected promise
		rejectedPromise: (_c) => {
			return Promise.reject(new UserError("promised rejection"));
		},
		// Action to check the collected results
		getResults: (c) => {
			return c.vars.results;
		},
	},
});
