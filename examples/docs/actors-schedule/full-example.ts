import { actor } from "rivetkit";

interface Reminder {
  userId: string;
  message: string;
  scheduledFor: number;
}

interface ReminderState {
  reminders: Record<string, Reminder>;
}

// Mock email function
function sendEmail(to: string, message: string) {
  console.log(`Sending email to ${to}: ${message}`);
}

const reminderService = actor({
  state: { reminders: {} } as ReminderState,

  actions: {
    setReminder: async (c, userId: string, message: string, delayMs: number) => {
      const reminderId = crypto.randomUUID();

      // Store the reminder in state
      c.state.reminders[reminderId] = {
        userId,
        message,
        scheduledFor: Date.now() + delayMs
      };

      // Schedule the sendReminder action to run after the delay
      await c.schedule.after(delayMs, "sendReminder", reminderId);

      return { reminderId };
    },

    sendReminder: (c, reminderId: string) => {
      const reminder = c.state.reminders[reminderId];
      if (!reminder) return;

      // Send reminder notification
      if (c.conns.size > 0) {
        // Send the reminder to all connected clients
        for (const conn of c.conns.values()) {
          conn.send("reminder", {
            message: reminder.message,
            scheduledAt: reminder.scheduledFor
          });
        }
      } else {
        // User is offline, send an email notification
        sendEmail(reminder.userId, reminder.message);
      }

      // Clean up the processed reminder
      delete c.state.reminders[reminderId];
    }
  }
});
