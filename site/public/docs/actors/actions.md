# Actions

Actions are how clients & other actors communicate with actors. Actions are defined as functions in the actor configuration and can be called from clients.

**Performance**

Actions are very lightweight. They can be called hundreds of times per second to send realtime data to the
actor.

## Writing Actions

Actions are defined in the `actions` object when creating a actor:

```typescript
const mathUtils = actor(,
  actions: 
  }
});
```

Each action receives a context object (commonly named `c`) as its first parameter, which provides access to state, connections, and other utilities. Additional parameters follow after that.

### Private Helper Functions

You can define helper functions outside the actions object to keep your code organized. These functions cannot be called directly by clients:

```typescript
// Private helper function - not callable by clients
const calculateFee = (amount) => ;

const paymentProcessor = actor(,
  actions: );
      return ;
    }
  }
});
```

### Streaming Return Data

Actions have a single return value. To stream realtime data in response to an action, use [events](/docs/actors/events).

## Calling Actions

Calling actions from the client is simple:

```typescript
const client = createClient("http://localhost:8080");
const counter = await client.counter.get();
const result = await counter.increment(42);
console.log(result); // The value returned by the action
```

Calling actions from the client are async and require an `await`, even if the action itself is not async.

### Type Safety

The actor client includes type safety out of the box. When you use `createClient()`, TypeScript automatically infers action parameter and return types:

```typescript src/index.ts
// Create simple counter
const counter = actor(,
  actions: 
  }
});

// Create and the app
const registry = setup(
});
```

```typescript client.ts
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

```typescript actor.ts
const user = actor(,
  actions: 
        });
      }
      
      // Rest of the user registration logic...
    }
  }
});
```

```typescript client.ts
try  catch (error) 
}
```

### Internal Errors

All other errors will return an error with the code `internal_error` to the client. This helps keep your application secure, as errors can sometimes expose sensitive information.

## Schema Validation

Data schemas are not validated by default. For production applications, use a library like [zod](https://zod.dev/) to validate input types.

For example, to validate action parameters:

```typescript
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

  Native runtime type validation is coming soon to Rivet.

## Authentication

By default, clients can call all actions on a actor without restriction. Make sure to implement authentication if needed. Documentation on authentication is available [here](/docs/general/authentication).

## Using `ActionContext` Type Externally

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

See [Helper Types](/docs/actors/helper-types) for more details on using `ActionContextOf` and other type utilities.