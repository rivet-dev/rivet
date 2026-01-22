# Actor Scheduling

> Source: `src/content/docs/actors/schedule.mdx`
> Canonical URL: https://rivet.gg/docs/actors/schedule
> Description: Schedule actor actions in the future with persistent timers that survive restarts and upgrades.

---
Scheduling is used to trigger events in the future. The actor scheduler is like `setTimeout`, except the timeout will persist even if the actor restarts, upgrades, or crashes.

## Use Cases

Scheduling is helpful for long-running timeouts like month-long billing periods or account trials.

## Scheduling

### `c.schedule.after(duration, actionName, ...args)`

Schedules a function to be executed after a specified duration. This function persists across actor restarts, upgrades, or crashes.

Parameters:

- `duration` (number): The delay in milliseconds.
- `actionName` (string): The name of the action to be executed.
- `...args` (unknown[]): Additional arguments to pass to the function.

### `c.schedule.at(timestamp, actionName, ...args)`

Schedules a function to be executed at a specific timestamp. This function persists across actor restarts, upgrades, or crashes.

Parameters:

- `timestamp` (number): The exact time in milliseconds since the Unix epoch when the function should be executed.
- `actionName` (string): The name of the action to be executed.
- `...args` (unknown[]): Additional arguments to pass to the function.

## Full Example

```typescript
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
    setReminder: (c, userId: string, message: string, delayMs: number) => {
      const reminderId = crypto.randomUUID();

      // Store the reminder in state
      c.state.reminders[reminderId] = {
        userId,
        message,
        scheduledFor: Date.now() + delayMs
      };

      // Schedule the sendReminder action to run after the delay
      c.schedule.after(delayMs, "sendReminder", reminderId);

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
```

_Source doc path: /docs/actors/schedule_
