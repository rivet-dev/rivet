import { actor, event } from "rivetkit";

const reminder = actor({
  state: { message: "Time to check your tasks." },
  events: {
    reminder: event<{ message: string }>(),
  },
  onCreate: async (c) => {
    // Install initial schedules once, when this actor is first created.
    await c.schedule.after(30_000, "sendReminder");

    await c.cron.every({
      name: "reminder-check",
      intervalMs: 60_000,
      action: "sendReminder",
    });
  },
  actions: {
    sendReminder: (c) => {
      c.broadcast("reminder", { message: c.state.message });
    },
  },
});
