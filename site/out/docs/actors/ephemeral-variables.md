# Ephemeral Variables

In addition to persisted state, Rivet provides a way to store ephemeral data that is not saved to permanent storage using `vars`. This is useful for temporary data that only needs to exist while the actor is running or data that cannot be serialized.

`vars` is designed to complement `state`, not replace it. Most actors should use both: `state` for critical business data and `vars` for ephemeral or non-serializable data.

## Initializing Variables

There are two ways to define an actor's initial vars:

Define an actor vars as a constant value:

```typescript
// Define vars as a constant
const counter = actor(,
  
  // Define ephemeral variables
  vars: ,
  
  actions: 
});
```

This value will be cloned for every new actor using `structuredClone`.

Create actor state dynamically on each actors' start:

```typescript
// Define vars with initialization logic
const counter = actor(,
  
  // Define vars using a creation function
  createVars: () => ;
  },
  
  actions: 
});
```

## Using Variables

Vars can be accessed and modified through the context object with `c.vars`:

```typescript
const counter = actor(,
  
  // Create ephemeral objects that won't be serialized
  createVars: () => `);
    });
    
    return ;
  },
  
  actions: 
  }
});
```

## When to Use `vars` vs `state`

In practice, most actors will use both: `state` for critical business data and `vars` for ephemeral or non-serializable data.

Use `vars` when:

- You need to store temporary data that doesn't need to survive restarts
- You need to maintain runtime-only references that can't be serialized (database connections, event emitters, class instances, etc.)

Use `state` when:

- The data must be preserved across actor sleeps, restarts, updates, or crashes
- The information is essential to the actor's core functionality and business logic