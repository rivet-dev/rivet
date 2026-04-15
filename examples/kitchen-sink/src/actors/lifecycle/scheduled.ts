import { actor, event } from "rivetkit";

export const scheduled = actor({
	state: {
		lastRun: 0,
		scheduledCount: 0,
		taskHistory: [] as string[],
	},
	events: {
		scheduled: event<{ time: number; count: number }>(),
		scheduledWithId: event<{ taskId: string; time: number; count: number }>(),
	},
	actions: {
		// Schedule using 'at' with specific timestamp
		scheduleTaskAt: (c, timestamp: number) => {
			c.schedule.at(timestamp, "onScheduledTask");
			return timestamp;
		},

		// Schedule using 'after' with delay
		scheduleTaskAfter: (c, delayMs: number) => {
			c.schedule.after(delayMs, "onScheduledTask");
			return Date.now() + delayMs;
		},

		// Schedule with a task ID for ordering tests
		scheduleTaskAfterWithId: (c, taskId: string, delayMs: number) => {
			c.schedule.after(delayMs, "onScheduledTaskWithId", taskId);
			return { taskId, scheduledFor: Date.now() + delayMs };
		},

		// Original method for backward compatibility
		scheduleTask: (c, delayMs: number) => {
			const timestamp = Date.now() + delayMs;
			c.schedule.at(timestamp, "onScheduledTask");
			return timestamp;
		},

		// Getters for state
		getLastRun: (c) => {
			return c.state.lastRun;
		},

		getScheduledCount: (c) => {
			return c.state.scheduledCount;
		},

		getTaskHistory: (c) => {
			return c.state.taskHistory;
		},

		clearHistory: (c) => {
			c.state.taskHistory = [];
			c.state.scheduledCount = 0;
			c.state.lastRun = 0;
			return true;
		},

		// Scheduled task handlers
		onScheduledTask: (c) => {
			c.state.lastRun = Date.now();
			c.state.scheduledCount++;
			c.broadcast("scheduled", {
				time: c.state.lastRun,
				count: c.state.scheduledCount,
			});
		},

		onScheduledTaskWithId: (c, taskId: string) => {
			c.state.lastRun = Date.now();
			c.state.scheduledCount++;
			c.state.taskHistory.push(taskId);
			c.broadcast("scheduledWithId", {
				taskId,
				time: c.state.lastRun,
				count: c.state.scheduledCount,
			});
		},
	},
});
