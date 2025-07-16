# Testing

Rivet provides a straightforward testing framework to build reliable and maintainable applications. This guide covers how to write effective tests for your actor-based services.

## Setup

To set up testing with Rivet:

```bash
# Install Vitest
npm install -D vitest

# Run tests
npm test
```

## Basic Testing Setup

Rivet includes a test helper called `setupTest` that configures a test environment with in-memory drivers for your actors. This allows for fast, isolated tests without external dependencies.

```ts tests/my-actor.test.ts
test("my actor test", async (test) =>  = await setupTest(test, app);
  
  // Now you can interact with your actor through the client
  const myActor = await client.myActor.get();
  
  // Test your actor's functionality
  await myActor.someAction();
  
  // Make assertions
  const result = await myActor.getState();
  expect(result).toEqual("updated");
});
```

```ts src/index.ts
const myActor = actor(,
  actions: ,
    getState: (c) => 
  }
});

const registry = setup(
});
```

## Testing Actor State

The test framework uses in-memory drivers that persist state within each test, allowing you to verify that your actor correctly maintains state between operations.

```ts tests/counter.test.ts
test("actor should persist state", async (test) =>  = await setupTest(test, app);
  const counter = await client.counter.get();
  
  // Initial state
  expect(await counter.getCount()).toBe(0);
  
  // Modify state
  await counter.increment();
  
  // Verify state was updated
  expect(await counter.getCount()).toBe(1);
});
```

```ts src/index.ts
const counter = actor(,
  actions: ,
    getCount: (c) => 
  }
});

const registry = setup(
});
```

## Testing Events

For actors that emit events, you can verify events are correctly triggered by subscribing to them:

```ts tests/chat-room.test.ts
test("actor should emit events", async (test) =>  = await setupTest(test, app);
  const chatRoom = await client.chatRoom.get();
  
  // Set up event handler with a mock function
  const mockHandler = vi.fn();
  chatRoom.on("newMessage", mockHandler);
  
  // Trigger the event
  await chatRoom.sendMessage("testUser", "Hello world");
  
  // Wait for the event to be emitted
  await vi.waitFor(() => );
});
```

```ts src/index.ts
const chatRoom = actor(,
  actions: );
      c.broadcast("newMessage", username, message);
    },
    getHistory: (c) => ,
  },
});

// Create and the app
const registry = setup(
});
```

## Testing Schedules

Rivet's schedule functionality can be tested using Vitest's time manipulation utilities:

```ts tests/scheduler.test.ts
test("scheduled tasks should execute", async (test) =>  = await setupTest(test, app);
  const scheduler = await client.scheduler.get();
  
  // Set up a scheduled task
  await scheduler.scheduleTask("reminder", 60000); // 1 minute in the future
  
  // Fast-forward time by 1 minute
  await vi.advanceTimersByTimeAsync(60000);
  
  // Verify the scheduled task executed
  expect(await scheduler.getCompletedTasks()).toContain("reminder");
});
```

```ts src/index.ts
const scheduler = actor(,
  actions: ;
    },
    completeTask: (c, taskName: string) => ;
    },
    getCompletedTasks: (c) => 
  }
});

const registry = setup(
});
```

The `setupTest` function automatically calls `vi.useFakeTimers()`, allowing you to control time in your tests with functions like `vi.advanceTimersByTimeAsync()`. This makes it possible to test scheduled operations without waiting for real time to pass.

## Best Practices

1. **Isolate tests**: Each test should run independently, avoiding shared state.
2. **Test edge cases**: Verify how your actor handles invalid inputs, concurrent operations, and error conditions.
3. **Mock time**: Use Vitest's timer mocks for testing scheduled operations.
4. **Use realistic data**: Test with data that resembles production scenarios.

Rivet's testing framework automatically handles server setup and teardown, so you can focus on writing effective tests for your business logic.