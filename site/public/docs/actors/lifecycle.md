# Lifecycle

Understand actor lifecycle hooks and initialization patterns

Actors follow a well-defined lifecycle with hooks at each stage. Understanding these hooks is essential for proper initialization, state management, and cleanup.

## Input Parameters

Actors can receive input parameters when created, allowing for flexible initialization:

### Defining Input Schema

Use Zod to define a schema for input validation:

```typescript
const gameActor = actor(),
  
  actions: ),
  },
});
```

### Passing Input to Actors

Input is provided when creating actor instances:

```typescript
// Client side - create with input
const game = await client.game.create(["game-123"], );

// getOrCreate can also accept input (used only if creating)
const gameHandle = client.game.getOrCreate(["game-456"], );
```

### Input in Lifecycle Hooks

Input is available in lifecycle hooks via the `opts` parameter:

```typescript
const chatRoom = actor(,
    messages: [],
  }),
  
  onCreate: (c, opts) => `);
    
    // Setup external services based on input
    if (opts.input?.isPrivate) 
  },
  
  actions: ),
  },
});
```

## Lifecycle Hooks

Actor lifecycle hooks are defined as functions in the actor configuration.

### `createState` and `state`

The `createState` function or `state` constant defines the initial state of the actor (see [state documentation](/docs/actors/state)). The `createState` function is called only once when the actor is first created.

### `createVars` and `vars`

The `createVars` function or `vars` constant defines ephemeral variables for the actor (see [state documentation](/docs/actors/state)). These variables are not persisted and are useful for storing runtime-only objects or temporary data.

The `createVars` function can also receive driver-specific context as its second parameter, allowing access to driver capabilities like Rivet KV or Cloudflare Durable Object storage.

```typescript
// Using vars constant
const counter1 = actor(,
  vars: ,
  actions: 
});

// Using createVars function
const counter2 = actor(,
  createVars: () => ;
  },
  actions: 
});

// Access driver-specific context
const exampleActor = actor(,
  // Access driver context in createVars
  createVars: (c, driverCtx) => (),
  actions: 
  }
});
```

### `onCreate`

The `onCreate` hook is called at the same time as `createState`, but unlike `createState`, it doesn't return any value. Use this hook for initialization logic that doesn't affect the initial state.

```typescript
// Using state constant
const counter1 = actor(,
  actions: 
});

// Using createState function
const counter2 = actor(;
  },
  actions: 
});

// Using onCreate
const counter3 = actor(,
  
  // Run initialization logic (logging, external service setup, etc.)
  onCreate: (c, opts) => ,
  
  actions: 
});
```

### `onStart`

This hook is called any time the actor is started (e.g. after restarting, upgrading code, or crashing).

This is called after the actor has been initialized but before any connections are accepted.

Use this hook to set up any resources or start any background tasks, such as `setInterval`.

```typescript
const counter = actor(,
  vars: ,
  
  onStart: (c) => , 10000);
    
    // Store interval ID in vars to clean up later if needed
    c.vars.intervalId = intervalId;
  },
  
  actions: 
    }
  }
});
```

### `onStateChange`

Called whenever the actor's state changes. This is often used to broadcast state updates.

```typescript
const counter = actor(,
  
  onStateChange: (c, newState) => );
  },
  
  actions: 
  }
});
```

### `createConnState` and `connState`

There are two ways to define the initial state for connections:
1. `connState`: Define a constant object that will be used as the initial state for all connections
2. `createConnState`: A function that dynamically creates initial connection state based on connection parameters

### `onBeforeConnect`

The `onBeforeConnect` hook is called whenever a new client connects to the actor. Clients can pass parameters when connecting, accessible via `params`. This hook is used for connection validation and can throw errors to reject connections.

The `onBeforeConnect` hook does NOT return connection state - it's used solely for validation.

```typescript
const chatRoom = actor(,
  
  // Method 1: Use a static default connection state
  connState: ,
  
  // Method 2: Dynamically create connection state
  createConnState: (c, ) => ;
  },
  
  // Validate connections before accepting them
  onBeforeConnect: (c, ) => 
    
    // Authentication is valid, connection will proceed
    // The actual connection state will come from connState or createConnState
  },
  
  actions: 
});
```

Connections cannot interact with the actor until this method completes successfully. Throwing an error will abort the connection. This can be used for authentication - see [Authentication](/docs/general/authentication) for details.

### `onConnect`

Executed after the client has successfully connected.

```typescript
const chatRoom = actor(, messages: [] },
  
  onConnect: (c) => ;
    
    // Broadcast that a user joined
    c.broadcast("userJoined", );
    
    console.log(`User $ connected`);
  },
  
  actions: 
});
```

Messages will not be processed for this actor until this hook succeeds. Errors thrown from this hook will cause the client to disconnect.

### `onDisconnect`

Called when a client disconnects from the actor. Use this to clean up any connection-specific resources.

```typescript
const chatRoom = actor(, messages: [] },
  
  onDisconnect: (c) => 
    
    // Broadcast that a user left
    c.broadcast("userLeft", );
    
    console.log(`User $ disconnected`);
  },
  
  actions: 
});
```

## Destroying Actors

Actors can be shut down gracefully with `c.shutdown()`. Clients will be gracefully disconnected.

```typescript
const temporaryRoom = actor(,
  
  createState: () => (),
  
  onStart: (c) =>  else , timeUntilExpiry);
    }
  },
  
  actions: );
      
      // Shutdown the actor
      c.shutdown();
    }
  }
});
```

This action is permanent and cannot be reverted.

## Using `ActorContext` Type Externally

When extracting logic from lifecycle hooks or actions into external functions, you'll often need to define the type of the context parameter. Rivet provides helper types that make it easy to extract and pass these context types to external functions.

```typescript
const myActor = actor(,
  
  // Use external function in lifecycle hook
  onStart: (c) => logActorStarted(c)
});

// Simple external function with typed context
function logActorStarted(c: ActorContextOf) `);
}
```

See [Helper Types](/docs/actors/helper-types) for more details on using `ActorContextOf`.

## Full Example

```typescript
const counter = actor(),
  
  // Initialize actor (run setup that doesn't affect initial state)
  onCreate: (c, opts) => " initialized`);
    // Set up external resources, logging, etc.
  },
  
  // Define default connection state
  connState: ,
  
  // Dynamically create connection state based on params
  createConnState: (c, ) => ;
  },
  
  // Lifecycle hooks
  onStart: (c) => " started with count:`, c.state.count);
  },
  
  onStateChange: (c, newState) => );
  },
  
  onBeforeConnect: (c, ) => 
    
    // Validate with your API and determine the user
    const authInfo = validateAuthToken(authToken);
    if (!authInfo) 
    
    // If validation succeeds, connection proceeds
    // Connection state will be set by createConnState
  },
  
  onConnect: (c) =>  connected to "$"`);
  },
  
  onDisconnect: (c) =>  disconnected from "$"`);
  },
  
  // Define actions
  actions: ,
    
    reset: (c) => 
      
      c.state.count = 0;
      return c.state.count;
    },
    
    getInfo: (c) => (),
  }
});

default counter;
```