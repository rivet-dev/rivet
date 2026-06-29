import { actor, queue, setup } from "rivetkit";

export const reminder = actor({
  state: {},
  queues: {
    notify: queue<{ userId: string }>(),
  },
  actions: {
    scheduleReminder: async (c, userId: string) => {
      // Enqueue a message in 30 seconds.
      c.schedule.after(30_000, "enqueueReminder", userId);
    },
    enqueueReminder: async (c, userId: string) => {
      await c.queue.send("notify", { userId });
    },
  },
  run: async (c) => {
    for await (const message of c.queue.iter()) {
      console.log("Sending reminder to", message.body.userId);
    }
  },
});

export const registry = setup({ use: { reminder } });
