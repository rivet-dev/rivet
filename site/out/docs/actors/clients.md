# Actor Clients

Learn how to call actions and connect to actors from client applications using Rivet's TypeScript client library.

Rivet also supports [React](/docs/clients/react) and [Rust](/docs/clients/rust) clients.

Using the RivetKit client is completely optional. If you prefer to write your own networking logic, you can either:

- Make HTTP requests directly to the registry (see [OpenAPI spec](/docs/clients/openapi))
- Write your own HTTP endpoints and use the client returned from `registry.runServer` (see below)

## Client Setup

There are several ways to create a client for communicating with actors:

		From your backend server that hosts the registry:

		```typescript }
		const  = registry.createServer();

		const app = new Hono();

		app.post("/foo", () => );

		serve(app);
		```

		This client doesn't [require authentication](/docs/actors/authentication).

		For frontend applications or external services connecting to your Rivet backend:

		```typescript }
		// IMPORTANT: Must use `type`

		const client = createClient("http://localhost:8080");

		const response = await client.otherActor.getOrCreate().foo();
		```

		Configure the client with additional options:

		```typescript }
		const client = createClient("http://localhost:8080", );
		```

		This client [requires authentication](/docs/actors/authentication).

		**Important**: Use `import type` when importing the registry type in order to
		avoid accidentally bundling your backend code.

		```typescript }
		// ✅ Do this
		// 🚫 Not this
		```

		From within an actor to communicate with other actors:

		```typescript }
		const myActor = actor(
			}
		});
		```

		This client doesn't [require authentication](/docs/actors/authentication).

		Read more about [communicating between actors](/docs/actors/communicating-between-actors).

## `ActorClient`

The `ActorClient` provides methods for finding and creating actors. All methods return an `ActorHandle` that you can use to call actions or establish connections.

### `get(key?, opts?)` - Find Existing Actor

Returns a handle to an existing actor or `null` if it doesn't exist:

```typescript
// Get existing actor by key
const handle = client.myActor.get(["actor-id"]);

if (handle)  else 
```

### `getOrCreate(key?, opts?)` - Find or Create Actor

Returns a handle to an existing actor or creates a new one if it doesn't exist:

```typescript
// Get or create actor (synchronous)
const counter = client.counter.getOrCreate(["my-counter"]);

// With initialization input
const game = client.game.getOrCreate(["game-123"], 
});

// Call actions immediately
const count = await counter.increment(5);
```

`get()` and `getOrCreate()` are synchronous and return immediately. The actor is created lazily when you first call an action.

### `create(key?, opts?)` - Create New Actor

Explicitly creates a new actor instance, failing if one already exists:

```typescript
// Create new actor (async)
const newGame = await client.game.create(["game-456"], 
});

// Actor is guaranteed to be newly created
await newGame.initialize();
```

### `getForId(id, opts?)` - Find by Internal ID

Connect to an actor using its internal system ID:

```typescript
// Connect by internal ID
const actorId = "55425f42-82f8-451f-82c1-6227c83c9372";
const actor = client.myActor.getForId(actorId);

await actor.performAction();
```

Prefer using keys over internal IDs for actor discovery. IDs are primarily for debugging and advanced use cases.

## `ActorHandle`

An `ActorHandle` represents a reference to an actor instance and provides methods for calling actions and establishing connections. You get an `ActorHandle` from the `ActorClient` methods like `get()`, `getOrCreate()`, and `create()`.

### Calling Actions

You can call actions directly on an `ActorHandle`:

```typescript
const counter = client.counter.getOrCreate(["my-counter"]);

// Call actions directly
const count = await counter.increment(5);
const currentValue = await counter.getCount();
await counter.reset();
```

Actions called on an `ActorHandle` are stateless - each call is independent and doesn't maintain a persistent connection to the actor.

### `fetch(input, init?)` - Raw HTTP Requests

Make direct HTTP requests to the actor's `onFetch` handler:

```typescript
const actor = client.myActor.getOrCreate(["key"]);

// GET request
const response = await actor.fetch("/api/hello", );
const data = await response.json();

// POST request with body
const postResponse = await actor.fetch("/api/echo", ,
  body: JSON.stringify()
});

// Can also pass a Request object
const request = new Request("/api/data", );
const requestResponse = await actor.fetch(request);
```

See [Fetch & WebSocket Handler](/docs/actors/fetch-and-websocket-handler) documentation for more information.

For most use cases, actions provide a higher-level API that's easier to work with than raw HTTP handlers.

### `websocket(path?, protocols?)` - Raw WebSocket Connections

Create direct WebSocket connections to the actor's `onWebSocket` handler:

```typescript
const actor = client.myActor.getOrCreate(["key"]);

// Basic WebSocket connection
const ws = await actor.websocket();
ws.addEventListener("message", (event) => );
ws.send("Hello WebSocket!");

// WebSocket with custom path
const streamWs = await actor.websocket("/stream");

// WebSocket with protocols
const protocolWs = await actor.websocket("/", ["chat", "v1"]);
```

See [Fetch & WebSocket Handler](/docs/actors/fetch-and-websocket-handler) documentation for more information.

For most use cases, actions & events provide a higher-level API that's easier to work with than raw HTTP handlers.

### `connect(params?)` - Establish Stateful Connection

To open a stateful connection using `ActorConn`, call `.connect()`:

```typescript
const counter = client.counter.getOrCreate(["live-counter"]);
const connection = counter.connect();

// Listen for events
connection.on("countChanged", (newCount: number) => );

// Call actions through the connection
const result = await connection.increment(1);

// Clean up when done
await connection.dispose();
```

## `ActorConn`

Real-time connections enable bidirectional communication between clients and actors through persistent connections. Rivet automatically negotiates between WebSocket (preferred for full duplex) and Server-Sent Events (SSE) as a fallback for restrictive environments.

For more information on connections, see the [connections documentation](/docs/actors/connections). For more information on handling events, see the [events documentation](/docs/actors/events).

### Calling Actions

You can also call actions through an `ActorConn`, just like with an `ActorHandle`:

```typescript
const connection = counter.connect();

// Call actions through the connection
const count = await connection.increment(5);
const currentValue = await connection.getCount();
```

### Reconnections

Connections automatically handle network failures with built-in reconnection logic:

- **Exponential backoff**: Retry delays increase progressively to avoid overwhelming the server
- **Action queuing**: Actions called while disconnected are queued and sent once reconnected
- **Event resubscription**: Event listeners are automatically restored on reconnection

### `on(eventName, callback)` - Listen for Events

Listen for events from the actor:

```typescript
// Listen for chat messages
connection.on("messageReceived", (message) => : $`);
});

// Listen for game state updates
connection.on("gameStateChanged", (gameState) => );

// Listen for player events
connection.on("playerJoined", (player) =>  joined the game`);
});
```

### `once(eventName, callback)` - Listen Once

Listen for an event only once:

```typescript
// Wait for game to start
connection.once("gameStarted", () => );
```

### `dispose()` - Clean Up Connection

Always dispose of connections when finished to free up resources:

```typescript
const connection = actor.connect();

try  finally 

// Or with automatic cleanup in React/frameworks
useEffect(() => ;
}, []);
```

**Important:** Disposing a connection:
- Closes the underlying WebSocket or SSE connection
- Removes all event listeners
- Cancels any pending reconnection attempts
- Prevents memory leaks in long-running applications

## Authentication

### Connection Parameters

Pass authentication data when connecting to actors:

```typescript
// With connection parameters
const chat = client.chatRoom.getOrCreate(["general"], 
});

const connection = chat.connect();

// Or for action calls
const result = await chat.sendMessage("Hello world!");
```

### onAuth Hook Validation

Actors can validate authentication using the `onAuth` hook:

```typescript
const protectedActor = actor( = opts;
    
    // Extract token from params or headers
    const token = params.authToken || req.headers.get("Authorization");
    
    if (!token) );
    }
    
    // Validate and return user data
    const user = await validateJWT(token);
    
    // Check permissions based on what the client is trying to do
    if (intents.has("create") && user.role !== "admin") );
    }
    
    return ;
  },
  
  actions:  = c.conn.auth;
      
      if (role !== "admin") );
      }
      
      return `Hello admin $`;
    }
  }
});
```

Learn more about [authentication patterns](/docs/actors/authentication).

## Type Safety

Rivet provides end-to-end type safety between clients and actors:

### Action Type Safety

TypeScript validates action signatures and return types:

```typescript
// TypeScript knows the action signatures
const counter = client.counter.getOrCreate(["my-counter"]);

const count: number = await counter.increment(5);   // ✓ Correct
const invalid = await counter.increment("5");       // ✗ Type error

// IDE autocomplete shows available actions
counter./*  */
```

### Client Type Safety

Import types from your registry for full type safety:

```typescript
// Client is fully typed
const client = createClient("http://localhost:8080");

// IDE provides autocomplete for all actors
client./*  */
```

## Best Practices

### `ActorHandle` (Stateless) vs `ActorConn` (Stateful) Clients

**Use `ActorHandle` (Stateless) For:**
- Simple request-response operations
- One-off operations  
- Server-side integration

```typescript
// Good for simple operations
const result = await counter.increment(1);
const status = await server.getStatus();
```

**Use `ActorConn` (Stateful) Connections For:**
- Real-time updates needed
- Event-driven interactions
- Long-lived client sessions

```typescript
// Good for real-time features
const connection = chatRoom.connect();
connection.on("messageReceived", updateUI);
await connection.sendMessage("Hello!");
```

### Resource Management

Always clean up connections when finished:

```typescript
// Manual cleanup
const connection = actor.connect();
try  finally 
```

### Error Handling

Handle connection errors using the `.onError()` method:

```typescript }
const connection = actor.connect();

connection.onError((error) =>  else if (error.code === 'ACTOR_NOT_FOUND') 
});
```

```tsx }
function ConnectionErrorHandler() );

  useEffect(() => , 5000);
    });

    // Clean up error handler when component unmounts
    return unsubscribe;
  }, [actor.connection]);

  // ...rest of component...
}
```

### Execute Actions in Parallel

You can execute batch requests in parallel:

```typescript
// Batch multiple operations through a connection
const connection = actor.connect();
await Promise.all([
  connection.operation1(),
  connection.operation2(),
  connection.operation3(),
]);

// Use getOrCreate for actors you expect to exist
const existing = client.counter.getOrCreate(["known-counter"]);

// Use create only when you need a fresh instance
const fresh = await client.counter.create(["new-counter"]);
```

However, it's recommended to move this logic to run within the actor instead of the client if executing multiple actions is a common pattern.