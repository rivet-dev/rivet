# Schedule

Scheduling is used to trigger events in the future. The actor scheduler is like `setTimeout`, except the timeout will persist even if the actor restarts, upgrades, or crashes.

## Use Cases

Scheduling is helpful for long-running timeouts like month-long billing periods or account trials.

## Scheduling

### `c.schedule.after(duration, fn, ...args)`

Schedules a function to be executed after a specified duration. This function persists across actor restarts, upgrades, or crashes.

Parameters:

- `duration` (number): The delay in milliseconds.
- `fn` (string): The name of the action to be executed.
- `...args` (unknown[]): Additional arguments to pass to the function.

### `c.schedule.at(timestamp, fn, ...args)`

Schedules a function to be executed at a specific timestamp. This function persists across actor restarts, upgrades, or crashes.

Parameters:

- `timestamp` (number): The exact time in milliseconds since the Unix epoch when the function should be executed.
- `fn` (string): The name of the action to be executed.
- `...args` (unknown[]): Additional arguments to pass to the function.

## Scheduling Private Actions

Currently, scheduling can only trigger public actions. If the scheduled action is private, it needs to be secured with something like a token.

## Full Example

```typescript
const reminderService = actor(
  },
  
  actions: ;
      
      // Schedule the sendReminder action to run after the delay
      c.after(delayMs, "sendReminder", reminderId);
      
      return ;
    },
    
    sendReminder: (c, reminderId) => );
      } else 
      
      // Clean up the processed reminder
      delete c.state.reminders[reminderId];
    }
  }
});
```