# Actions

Actions are how your backend, frontend, or other actors can communicate with actors. Actions are defined as functions in the actor configuration and can be called from clients.

Actions are very lightweight. They can be called thousands of times per second safely.

Actions are executed via HTTP requests or via WebSockets if [using `.connect()`](/docs/actors/connections).

For advanced use cases that require direct access to HTTP requests or WebSocket connections, see [raw HTTP and WebSocket handling](/docs/actors/fetch-and-websocket-handler).

## Writing Actions

Actions are defined in the `actions` object when creating an actor:

```typescript
const mathUtils = actor(,
  actions: 
  }
});
```

Each action receives a context object (commonly named `c`) as its first parameter, which provides access to state, connections, and other utilities. Additional parameters follow after that.

## Calling Actions

Actions can be called in different ways depending on your use case:

```typescript }
const client = createClient("http://localhost:8080");
const counter = await client.counter.getOrCreate();
const result = await counter.increment(42);
console.log(result); // The value returned by the action
```

Learn more about [communicating with actors from the frontend](/docs/actors/communicating-between-actors).

```typescript }
const registry = setup(
});

const  = registry.runServer();

// Use the client to call actions
const counter = await client.counter.getOrCreate();
const result = await counter.increment(42);
console.log(result);
```

Learn more about [communicating with actors from the backend](/docs/actors/communicating-between-actors).

```typescript }
const actorA = actor(,
  actions: 
  }
});
```

Learn more about [communicating between actors](/docs/actors/communicating-between-actors).

Calling actions from the client are async and require an `await`, even if the action itself is not async.

### Type Safety

The actor client includes type safety out of the box. When you use `createClient()`, TypeScript automatically infers action parameter and return types:

```typescript }
// Create simple counter
const counter = actor(,
  actions: 
  }
});

// Create and the app
const registry = setup(
});
```

```typescript }
const client = createClient("http://localhost:8080");

// Type-safe client usage
const counter = await client.counter.get();
await counter.increment(123); // OK
await counter.increment("non-number type"); // TypeScript error
await counter.nonexistentMethod(123); // TypeScript error
```

## Error Handling

Actors provide robust error handling out of the box for actions.

### User Errors

`UserError` can be used to return rich error data to the client. You can provide:

-   A human-readable message
-   A machine-readable code that's useful for matching errors in a try-catch (optional)
-   A metadata object for providing richer error context (optional)

For example:

```typescript }
const user = actor(,
  actions: 
        });
      }
      
      // Rest of the user registration logic...
    }
  }
});
```

```typescript }
try  catch (error) 
}
```

### Internal Errors

All other errors will return an error with the code `internal_error` to the client. This helps keep your application secure, as errors can sometimes expose sensitive information.

## Schema Validation

If passing data to an actor from the frontend, use a library like [Zod](https://zod.dev/) to validate input data.

For example, to validate action parameters:

```typescript }
// Define schema for action parameters
const IncrementSchema = z.object();

const counter = actor(,
  actions:  = IncrementSchema.parse(params);
        c.state.count += count;
        return c.state.count;
      } catch (err) 
        });
      }
    }
  }
});
```

## Authentication

By default, actors' actions are only accessible from your server-side client.

In order to expose actions publicly to the external client, you'll need to define `onAuth`. More documentation on authentication is available [here](/docs/actors/authentication). Read more about the [types of clients](/docs/actors/clients)

## Streaming Return Data

Actions have a single return value. To stream realtime data in response to an action, use [events](/docs/actors/events).

## Using `ActionContext` Externally

When writing complex logic for actions, you may want to extract parts of your implementation into separate helper functions. When doing this, you'll need a way to properly type the context parameter.

Rivet provides the `ActionContextOf` utility type for exactly this purpose:

```typescript
const counter = actor(,
  
  actions: 
  }
});

// Simple helper function with typed context
function incrementCount(c: ActionContextOf) 
```

See [helper types](/docs/actors/helper-types) for more details on using `ActionContextOf` and other utility types.