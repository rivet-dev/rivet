import { actor } from "rivetkit";

export const reminderAgent = actor({
	actions: {
		setUpSchedules: async (c) => {
			// this.schedule(60, ...): a delay in milliseconds, not seconds
			await c.schedule.after(60_000, "sendReminder", "Check email");

			// this.schedule(new Date(...), ...): a Unix timestamp in milliseconds
			await c.schedule.at(
				Date.parse("2026-12-01T09:00:00Z"),
				"sendReminder",
				"Monthly report due",
			);

			// this.schedule("0 8 * * *", ...): a named cron job
			await c.cron.set({
				name: "daily-digest",
				expression: "0 8 * * *",
				action: "dailyDigest",
			});

			// this.scheduleEvery(30, ...): a named fixed interval
			await c.cron.every({
				name: "poll",
				interval: 30_000,
				action: "poll",
			});
		},

		// Like cancelSchedule(id)
		cancelReminder: async (c, id: string) => c.schedule.cancel(id),

		// Scheduled callbacks are ordinary actions
		sendReminder: (_c, message: string) => {
			console.log("reminder:", message);
		},
		dailyDigest: (_c) => {
			console.log("sending the daily digest");
		},
		poll: (_c) => {
			console.log("polling");
		},
	},
});
