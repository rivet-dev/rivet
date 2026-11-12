import { actor, event, type ScheduledFireInfo, setup } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

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

const reminderActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS reminders (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL UNIQUE,
					schedule_id TEXT NOT NULL,
					message TEXT NOT NULL,
					scheduled_at INTEGER NOT NULL,
					completed_at INTEGER
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS daily_report (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					last_report_at INTEGER NOT NULL,
					last_report TEXT
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO daily_report (id, last_report_at) VALUES (1, 0)",
			);
		},
	}),
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
		sendDailyReport: async (c, fire: ScheduledFireInfo) => {
			const { lastReportAt: from, lastReport } = await loadDailyReport(c.db);
			const to = fire.scheduledAt;

			// Skip a run whose window was already reported.
			if (to <= from) return lastReport;

			const [counts] = (await c.db.execute(
				`SELECT
					COUNT(CASE WHEN completed_at >= ? AND completed_at < ? THEN 1 END) AS completed,
					COUNT(CASE WHEN completed_at IS NULL THEN 1 END) AS pending
				FROM reminders`,
				from,
				to,
			)) as { completed: number; pending: number }[];
			const report: DailyReport = {
				from,
				to,
				completed: counts.completed,
				pending: counts.pending,
			};

			await c.db.execute(
				"UPDATE daily_report SET last_report_at = ?, last_report = ? WHERE id = 1",
				to,
				JSON.stringify(report),
			);
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
				lastReport: (await loadDailyReport(c.db)).lastReport,
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

			await insertReminder(c.db, reminder);

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

			await insertReminder(c.db, reminder);

			return reminder;
		},

		// Trigger a scheduled reminder
		triggerReminder: async (c, reminderId: string) => {
			const reminder = await findReminder(c.db, reminderId);
			if (!reminder) {
				console.warn(`reminder not found: ${reminderId}`);
				return;
			}

			// Mark as completed
			reminder.completedAt = Date.now();
			await c.db.execute(
				"UPDATE reminders SET completed_at = ? WHERE id = ?",
				reminder.completedAt,
				reminder.id,
			);

			// Broadcast event
			c.broadcast("reminderTriggered", reminder);

			console.log(`reminder triggered: ${reminder.message}`);
		},
		// docs:end one-shot

		// Get all reminders
		getReminders: async (c): Promise<Reminder[]> => {
			const rows = (await c.db.execute(
				`SELECT ${REMINDER_COLUMNS} FROM reminders ORDER BY seq`,
			)) as ReminderRow[];
			return rows.map(toReminder);
		},

		// docs:start cancel
		// Cancel a pending scheduled reminder.
		cancelReminder: async (c, reminderId: string) => {
			const reminder = await findReminder(c.db, reminderId);
			if (!reminder) return { success: false };

			const cancelled = await c.schedule.cancel(reminder.scheduleId);
			if (cancelled) {
				await c.db.execute("DELETE FROM reminders WHERE id = ?", reminderId);
			}

			return { success: cancelled };
		},
		// docs:end cancel

		// Get statistics about reminders
		getStats: async (c) => {
			const [counts] = (await c.db.execute(
				"SELECT COUNT(*) AS total, COUNT(completed_at) AS completed FROM reminders",
			)) as { total: number; completed: number }[];
			const { total, completed } = counts;
			const pending = total - completed;

			return {
				total,
				completed,
				pending,
			};
		},
	},
});

type ReminderRow = {
	id: string;
	schedule_id: string;
	message: string;
	scheduled_at: number;
	completed_at: number | null;
};

const REMINDER_COLUMNS = "id, schedule_id, message, scheduled_at, completed_at";

function toReminder(row: ReminderRow): Reminder {
	return {
		id: row.id,
		scheduleId: row.schedule_id,
		message: row.message,
		scheduledAt: row.scheduled_at,
		completedAt: row.completed_at ?? undefined,
	};
}

async function insertReminder(db: RawAccess, reminder: Reminder) {
	await db.execute(
		"INSERT INTO reminders (id, schedule_id, message, scheduled_at) VALUES (?, ?, ?, ?)",
		reminder.id,
		reminder.scheduleId,
		reminder.message,
		reminder.scheduledAt,
	);
}

async function findReminder(
	db: RawAccess,
	reminderId: string,
): Promise<Reminder | undefined> {
	const rows = (await db.execute(
		`SELECT ${REMINDER_COLUMNS} FROM reminders WHERE id = ?`,
		reminderId,
	)) as ReminderRow[];
	return rows[0] ? toReminder(rows[0]) : undefined;
}

/** Reads the end of the last reported window and the last report, if any. */
async function loadDailyReport(
	db: RawAccess,
): Promise<{ lastReportAt: number; lastReport?: DailyReport }> {
	const [row] = (await db.execute(
		"SELECT last_report_at, last_report FROM daily_report WHERE id = 1",
	)) as { last_report_at: number; last_report: string | null }[];
	return {
		lastReportAt: row.last_report_at,
		lastReport: row.last_report
			? (JSON.parse(row.last_report) as DailyReport)
			: undefined,
	};
}

// docs:start registry
export const registry = setup({
	use: { reminderActor },
});
// docs:end registry

export type Registry = typeof registry;
export type { DailyReport, Reminder };

registry.start();
