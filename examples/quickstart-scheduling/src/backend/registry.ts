import { actor, setup } from "rivetkit";

interface Reminder {
	id: string;
	message: string;
	scheduledAt: number;
	completedAt?: number;
}

interface ReminderActorState {
	reminders: Reminder[];
	completedCount: number;
	healthCheckCount: number;
}

const reminderActor = actor({
	state: {
		reminders: [] as Reminder[],
		completedCount: 0,
		healthCheckCount: 0,
	} satisfies ReminderActorState as ReminderActorState,

	onCreate: async (c) => {
		// Schedule a recurring health check action every 5 seconds
		c.schedule.after(5000, "healthCheck");
	},

	onStart: (c) => {
		console.log("reminder actor started");
	},

	actions: {
		// Schedule a reminder with a delay in milliseconds
		scheduleReminder: (c, message: string, delayMs: number) => {
			const reminder: Reminder = {
				id: `reminder-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
				message,
				scheduledAt: Date.now() + delayMs,
			};

			c.state.reminders.push(reminder);
			c.schedule.after(delayMs, "triggerReminder", reminder.id);

			return reminder;
		},

		// Schedule a reminder at a specific timestamp
		scheduleReminderAt: (c, message: string, timestamp: number) => {
			const reminder: Reminder = {
				id: `reminder-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
				message,
				scheduledAt: timestamp,
			};

			c.state.reminders.push(reminder);
			c.schedule.at(timestamp, "triggerReminder", reminder.id);

			return reminder;
		},

		// Trigger a scheduled reminder
		triggerReminder: (c, reminderId: string) => {
			const reminder = c.state.reminders.find((r) => r.id === reminderId);
			if (!reminder) {
				console.warn(`reminder not found: ${reminderId}`);
				return;
			}

			// Mark as completed
			reminder.completedAt = Date.now();
			c.state.completedCount++;

			// Broadcast event
			c.broadcast("reminderTriggered", reminder);

			console.log(`reminder triggered: ${reminder.message}`);
		},

		// Get all reminders
		getReminders: (c) => {
			return c.state.reminders;
		},

		// Cancel a scheduled reminder
		// Note: Rivet doesn't currently support canceling scheduled actions
		// This will only remove the reminder from state
		cancelReminder: (c, reminderId: string) => {
			// Remove from state
			c.state.reminders = c.state.reminders.filter(
				(r) => r.id !== reminderId,
			);

			return {
				success: true,
				message:
					"reminder removed from state (note: scheduled action may still fire)",
			};
		},

		// Get statistics about reminders
		getStats: (c) => {
			const total = c.state.reminders.length;
			const completed = c.state.completedCount;
			const pending = total - completed;

			return {
				total,
				completed,
				pending,
				healthCheckCount: c.state.healthCheckCount,
			};
		},

		// Health check action (recurring)
		healthCheck: (c) => {
			c.state.healthCheckCount++;
			console.log(`health check #${c.state.healthCheckCount}`);

			// Schedule the next health check
			c.schedule.after(5000, "healthCheck");
		},
	},
});

export const registry = setup({
	use: { reminderActor },
});

export type Registry = typeof registry;
export type { Reminder };
