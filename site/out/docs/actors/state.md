# State

Actor state provides the best of both worlds: it's stored in-memory and persisted automatically. This lets you work with the data without added latency while still being able to survive crashes & upgrades.

Actors can also be used with external SQL databases. This can be useful to integrate actors with existing
applications or for storing relational data. Read more [here](/docs/actors/external-sql).

## Initializing State

There are two ways to define an actor's initial state:

Define an actor state as a constant value:

```typescript
// Simple state with a constant
const counter = actor(,
  
  actions: 
});
```

This value will be cloned for every new actor using `structuredClone`.

Create actor state dynamically on each actors' creation:

```typescript
// State with initialization logic
const counter = actor(;
  },
  
  actions: 
});
```

The `createState` function is called once when the actor is first created. See [Lifecycle](/docs/actors/lifecycle) for more details.

## Modifying State

To update state, modify the `state` property on the context object (`c.state`) in your actions:

```typescript
const counter = actor(,
  
  actions: ,
    
    add: (c, value) => 
  }
});
```

Only state stored in the `state` object will be persisted. Any other variables or properties outside of this are not persisted.

## State Saves

Actors automatically handle persisting state transparently. This happens at the end of every action if the state has changed. State is also automatically saved after `onFetch` and `onWebSocket` handlers finish executing.

For `onWebSocket` handlers specifically, you'll need to manually save state using `c.saveState()` while the WebSocket connection is open if you want state changes to be persisted immediately. This is because WebSocket connections can remain open for extended periods, and state changes made during event handlers (like `message` events) won't be automatically saved until the connection closes.

In other cases where you need to force a state change mid-action, you can use `c.saveState()`. This should only be used if your action makes an important state change that needs to be persisted before the action completes.

```typescript
const criticalProcess = actor(,
  
  actions: `);
      
      // Force save state before the async operation
      c.saveState();
      
      // Long-running operation that might fail
      await someRiskyOperation();
      
      // Update state again
      c.state.steps.push(`Completed step $`);
      
      return c.state.currentStep;
    }
  }
});
```

## State Isolation

Each actor's state is completely isolated, meaning it cannot be accessed directly by other actors or clients.

To interact with an actor's state, you must use [Actions](/docs/actors/actions). Actions provide a controlled way to read from and write to the state.

If you need a shared state between multiple actors, see [sharing and joining state](/docs/actors/sharing-and-joining-state).

## Ephemeral Variables

In addition to persisted state, actors can store ephemeral data that is not saved to permanent storage using `vars`. This is useful for temporary data or non-serializable objects like database connections or event emitters.

For complete documentation on ephemeral variables, see [Ephemeral Variables](/docs/actors/ephemeral-variables).

## Limitations

State is currently constrained to the available memory on the machine.

Only JSON-serializable types can be stored in state. In serverless runtimes that support it (Rivet, Cloudflare Workers), state is persisted under the hood in a compact, binary format. This is because JavaScript classes cannot be serialized & deserialized.

SQLite in Rivet Actors (coming soon) will provide a way of dynamically querying data with in-memory performance without being constrained to memory limits.