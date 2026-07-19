import { actor, event, setup } from "rivetkit";

interface Reminder {
	id: string;
	scheduleId: string;
	message: string;
	scheduledAt: number;
	completedAt?: number;
}

interface ReminderActorState {
	reminders: Reminder[];
	completedCount: number;
}

const reminderActor = actor({
	state: {
		reminders: [] as Reminder[],
		completedCount: 0,
	} satisfies ReminderActorState as ReminderActorState,
	events: {
		reminderTriggered: event<Reminder>(),
	},

	actions: {
		// Schedule a reminder with a delay in milliseconds
		scheduleReminder: async (c, message: string, delayMs: number) => {
			const id = `reminder-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
			const scheduleId = await c.schedule.after(
				delayMs,
				"triggerReminder",
				id,
			);
			const reminder: Reminder = {
				id,
				scheduleId,
				message,
				scheduledAt: Date.now() + delayMs,
			};

			c.state.reminders.push(reminder);

			return reminder;
		},

		// Schedule a reminder at a specific timestamp
		scheduleReminderAt: async (c, message: string, timestamp: number) => {
			const id = `reminder-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
			const scheduleId = await c.schedule.at(
				timestamp,
				"triggerReminder",
				id,
			);
			const reminder: Reminder = {
				id,
				scheduleId,
				message,
				scheduledAt: timestamp,
			};

			c.state.reminders.push(reminder);

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
		getReminders: (c): Reminder[] => {
			return c.state.reminders;
		},

		// Cancel a pending scheduled reminder.
		cancelReminder: async (c, reminderId: string) => {
			const reminder = c.state.reminders.find((r) => r.id === reminderId);
			if (!reminder) return { success: false };

			const cancelled = await c.schedule.cancel(reminder.scheduleId);
			if (cancelled) {
				c.state.reminders = c.state.reminders.filter(
					(r) => r.id !== reminderId,
				);
			}

			return { success: cancelled };
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
			};
		},
	},
});

export const registry = setup({
	use: { reminderActor },
});

export type Registry = typeof registry;
export type { Reminder };

registry.start();
