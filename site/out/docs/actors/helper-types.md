# Helper Types

Rivet provides several TypeScript helper types to make it easier to work with actors in a type-safe way.

## `Context` Types

When working with actors, you often need to access the context object. Rivet provides helper types to extract the context types from actor definitions.

### `ActorContextOf`

Extracts the full actor context type from an actor definition. This is the type of the context object (`c`) available in [lifecycle hooks](/docs/actors/lifecycle) and in [actions](/docs/actors/actions).

```typescript
const chatRoom = actor(,
  actions: ,
  actions: 
  }
});

// Now you can use this type elsewhere
function logMessage(context: ActorContextOf, message: string) );
}
```

### `ActionContextOf`

Extracts the action context type from an actor definition. This is the type of the context object (`c`) available in [actions](/docs/actors/actions). This cannot be used in [lifecycle hooks](/docs/actors/lifecycle).

```typescript
const counterWithProcessing = actor(,
  actions: 
  }
});

function processCounterAction(context: ActionContextOf) 
```