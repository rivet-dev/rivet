# Input Parameters

Pass initialization data to actors when creating instances

Actors can receive input parameters when created, allowing for flexible initialization and configuration. Input is passed during actor creation and is available in lifecycle hooks.

## Passing Input to Actors

Input is provided when creating actor instances using the `input` property:

```typescript
// Client side - create with input
const game = await client.game.create(["game-123"], 
});

// getOrCreate can also accept input (used only if creating)
const gameHandle = client.game.getOrCreate(["game-456"], 
});
```

## Accessing Input in Lifecycle Hooks

Input is available in lifecycle hooks via the `opts.input` parameter:

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

## Input Validation

You can validate input parameters in the `createState` or `onCreate` hooks:

```typescript
const GameInputSchema = z.object();

const game = actor(,
      gameState: "waiting",
    };
  },
  
  actions: ),
  },
});
```

## Input vs Connection Parameters

Input parameters are different from connection parameters:

- **Input**:
  - Passed when creating the actor instance
  - Use for actor-wide configuration
  - Available in lifecycle hooks
- **Connection parameters**:
  - Passed when connecting to an existing actor
  - Used for connection-specific configuration
  - Available in connection hooks

```typescript
// Actor creation with input
const room = await client.chatRoom.create(["room-123"], ,
  params: 
});
```

## Input Best Practices

### Use Type Safety

Define input types to ensure type safety:

```typescript
interface GameInput 

const game = actor() => (),
  
  actions: ,
});
```

### Store Input in State

If you need to access input data in actions, store it in the actor's state:

```typescript
const game = actor(,
    // Runtime state
    players: ,
    gameState: "waiting",
  }),
  
  actions: ,
  },
});
```