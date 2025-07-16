# Helper Types

Rivet provides several TypeScript helper types to make it easier to work with actors in a type-safe way.

## `Context` Types

When working with actors, you often need to access the context object. Rivet provides helper types to extract the context types from actor definitions.

### `ActorContextOf`

Extracts the full actor context type from a actor definition. This is the type of the context object (`c`) available in lifecycle hooks such as `onCreate`, `onStart`, etc.

```typescript
const chatRoom = actor(,
  actions: 
  }
});

// Extract the chat room context type
type ChatRoomContext = ActorContextOf;

// Now you can use this type elsewhere
function processChatRoomContext(context: ChatRoomContext) );
}
```

### `ActionContextOf`

Extracts the action context type from a actor definition. This is the type of the context object (`c`) available in action handlers.

```typescript
const counter = actor(,
  actions: 
  }
});

// Extract the action context type
type CounterActionContext = ActionContextOf;

// Use in other functions that need to work with the action context
function processCounterAction(context: CounterActionContext) 
```