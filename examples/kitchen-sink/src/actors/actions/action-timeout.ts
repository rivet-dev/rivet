import { actor } from "rivetkit";

// Short timeout actor
export const shortTimeoutActor = actor({
	options: {
		actionTimeout: 50, // 50ms timeout
	},
	actions: {
		quickAction: async (_c) => {
			return "quick response";
		},
		slowAction: async (_c) => {
			// This action should timeout
			await new Promise((resolve) => setTimeout(resolve, 100));
			return "slow response";
		},
	},
});

// Long timeout actor
export const longTimeoutActor = actor({
	options: {
		actionTimeout: 200, // 200ms timeout
	},
	actions: {
		delayedAction: async (_c) => {
			// This action should complete within timeout
			await new Promise((resolve) => setTimeout(resolve, 100));
			return "delayed response";
		},
	},
});

// Default timeout actor
export const defaultTimeoutActor = actor({
	actions: {
		normalAction: async (_c) => {
			await new Promise((resolve) => setTimeout(resolve, 50));
			return "normal response";
		},
	},
});

// Sync actor (timeout shouldn't apply)
export const syncTimeoutActor = actor({
	options: {
		actionTimeout: 50, // 50ms timeout
	},
	actions: {
		syncAction: (_c) => {
			return "sync response";
		},
	},
});
