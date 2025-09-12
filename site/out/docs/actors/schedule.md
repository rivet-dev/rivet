# Schedule

Scheduling is used to trigger events in the future. The actor scheduler is like `setTimeout`, except the timeout will persist even if the actor restarts, upgrades, or crashes.

	Scheduling is supported on the Rivet Cloud, Cloudflare Workers, file system, and memory drivers.

	Follow [this issue](https://github.com/rivet-gg/rivetkit/issues/1095) for Redis support.

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
const reminderService = actor(
  },
  
  actions: ;
      
      // Schedule the sendReminder action to run after the delay
      c.schedule.after(delayMs, "sendReminder", reminderId);
      
      return ;
    },
    
    sendReminder: (c, reminderId) => );
        });
      } else 
      
      // Clean up the processed reminder
      delete c.state.reminders[reminderId];
    }
  }
});
```