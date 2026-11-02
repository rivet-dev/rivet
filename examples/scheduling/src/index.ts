import { actor, event, type ScheduledFireInfo, setup } from "rivetkit";

// docs:start state
interface Reminder {
	id: string;
	scheduleId: string;
	message: string;
	scheduledAt: number;
	completedAt?: number;
}

interface DailyReport {
	from: number;
	to: number;
	completed: number;
	pending: number;
}

interface ReminderActorState {
	reminders: Reminder[];
	completedCount: number;
	lastReportAt: number;
	lastReport?: DailyReport;
}

const reminderActor = actor({
	state: {
		reminders: [] as Reminder[],
		completedCount: 0,
		lastReportAt: 0,
	} satisfies ReminderActorState as ReminderActorState,
	events: {
		reminderTriggered: event<Reminder>(),
		dailyReport: event<DailyReport>(),
	},
	// docs:end state

	// docs:start install
	// Install the daily report job when the Actor is created.
	onCreate: async (c) => {
		await c.cron.set({
			name: "daily-report",
			expression: "0 9 * * *",
			timezone: "America/Los_Angeles",
			action: "sendDailyReport",
		});
	},
	// docs:end install

	actions: {
		// docs:start report
		// Called by the daily-report cron job every day at 9:00.
		sendDailyReport: (c, fire: ScheduledFireInfo) => {
			const from = c.state.lastReportAt;
			const to = fire.scheduledAt;

			// Skip a run whose window was already reported.
			if (to <= from) return c.state.lastReport;

			const report: DailyReport = {
				from,
				to,
				completed: c.state.reminders.filter(
					(r) =>
						r.completedAt !== undefined &&
						r.completedAt >= from &&
						r.completedAt < to,
				).length,
				pending: c.state.reminders.filter(
					(r) => r.completedAt === undefined,
				).length,
			};

			c.state.lastReportAt = to;
			c.state.lastReport = report;
			c.broadcast("dailyReport", report);
			return report;
		},
		// docs:end report

		// docs:start read
		// Return the last report, the next run time, and recent runs.
		getDailyReport: async (c) => {
			const job = await c.cron.get("daily-report");
			const history = await c.cron.history("daily-report", { limit: 7 });
			return {
				lastReport: c.state.lastReport,
				nextRunAt: job?.nextRunAt,
				history,
			};
		},
		// docs:end read

		// docs:start one-shot
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
		// docs:end one-shot

		// Get all reminders
		getReminders: (c): Reminder[] => {
			return c.state.reminders;
		},

		// docs:start cancel
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
		// docs:end cancel

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

// docs:start registry
export const registry = setup({
	use: { reminderActor },
});
// docs:end registry

export type Registry = typeof registry;
export type { DailyReport, Reminder };

registry.start();
