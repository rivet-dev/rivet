import { actor } from "rivetkit";

export const reminder = actor({
	actions: {
		// docs:start schedule
		// ctx.storage.setAlarm(time): schedule an action instead of one alarm().
		// The schedule stores its arguments, so there's nothing to keep in storage.
		remind: async (c, message: string, delayMs: number) => {
			// The schedule ID doubles as the reminder ID
			return await c.schedule.after(delayMs, "sendReminder", message);
		},

		// Runs when the schedule fires. This replaces the alarm() handler.
		sendReminder: (c, message: string) => {
			// Scheduled actions are regular actions; reject direct client calls
			if (!c.scheduledFire)
				throw new Error("only callable by the schedule");
			console.log("reminder:", message);
		},

		// ctx.storage.deleteAlarm()
		cancel: async (c, id: string) => {
			return await c.schedule.cancel(id);
		},

		// ctx.storage.getAlarm()
		pending: async (c) => {
			return await c.schedule.list();
		},
		// docs:end schedule

		// docs:start cron
		// A recurring alarm that re-arms itself becomes a cron job
		startDailyDigest: async (c) => {
			await c.cron.set({
				name: "daily-digest",
				expression: "0 9 * * *",
				action: "sendDigest",
			});
		},

		sendDigest: (c) => {
			if (!c.scheduledFire)
				throw new Error("only callable by the schedule");
			console.log("sending digest");
		},
		// docs:end cron
	},
});
