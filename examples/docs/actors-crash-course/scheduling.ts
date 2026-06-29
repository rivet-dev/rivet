import { actor, event } from "rivetkit";

const reminder = actor({
	state: { message: "" },
	events: {
		reminder: event<{ message: string }>(),
	},
	actions: {
		// Schedule action to run after delay (ms)
		setReminder: (c, message: string, delayMs: number) => {
			c.state.message = message;
			c.schedule.after(delayMs, "sendReminder");
		},
		// Schedule action to run at specific timestamp
		setReminderAt: (c, message: string, timestamp: number) => {
			c.state.message = message;
			c.schedule.at(timestamp, "sendReminder");
		},
		sendReminder: (c) => {
			c.broadcast("reminder", { message: c.state.message });
		},
	},
});
