# Communicating with Actors

Learn how to call actions and connect to actors from client applications

This guide covers how to connect to and interact with actors from client applications using Rivet's JavaScript/TypeScript client library.

## Client Setup

### Creating a Client

There are several ways to create a client for communicating with actors:

		For frontend applications or external services connecting to your Rivet backend:

		```typescript
		const client = createClient("http://localhost:8080");
		```

		This client communicates over HTTP/WebSocket and requires authentication.

		From your backend server that hosts the registry:

		```typescript
		const  = registry.createServer();
		```

		This client bypasses network calls and doesn't require authentication.

		From within an actor to communicate with other actors:

		```typescript
		const myActor = actor(
			}
		});
		```

		Read more about [communicating between actors](/docs/actors/communicating-between-actors).

### Client Configuration

Configure the client with additional options:

```typescript
const client = createClient("http://localhost:8080", );
```

## Actor Handles

### `get(tags, opts?)` - Find Existing Actor

Returns a handle to an existing actor or `null` if it doesn't exist:

```typescript
// Get existing actor by tags
const handle = client.myActor.get(["actor-id"]);

if (handle)  else 
```

### `getOrCreate(tags, input?, opts?)` - Find or Create Actor

Returns a handle to an existing actor or creates a new one if it doesn't exist:

```typescript
// Get or create actor (synchronous)
const counter = client.counter.getOrCreate(["my-counter"]);

// With initialization input
const game = client.game.getOrCreate(["game-123"], );

// Call actions immediately
const count = await counter.increment(5);
```

`get()` and `getOrCreate()` are synchronous and return immediately. The actor is created lazily when you first call an action.

### `create(tags, input?, opts?)` - Create New Actor

Explicitly creates a new actor instance, failing if one already exists:

```typescript
// Create new actor (async)
const newGame = await client.game.create(["game-456"], );

// Actor is guaranteed to be newly created
await newGame.initialize();
```

### `getWithId(id, opts?)` - Find by Internal ID

Connect to an actor using its internal system ID:

```typescript
// Connect by internal ID
const actorId = "55425f42-82f8-451f-82c1-6227c83c9372";
const actor = client.myActor.getWithId(actorId);

await actor.performAction();
```

Prefer using tags over internal IDs for actor discovery. IDs are primarily for debugging and advanced use cases.

## Actions

### Calling Actions

Once you have an actor handle, call actions directly. All action calls are async:

```typescript
const counter = client.counter.getOrCreate(["my-counter"]);

// Call action with no arguments
const currentCount = await counter.getCount();

// Call action with arguments
const newCount = await counter.increment(5);

// Call action with object parameter
await counter.updateSettings();
```

### Action Parameters

Actions receive parameters exactly as defined in the actor:

```typescript
// Actor definition
const chatRoom = actor(
  }
});

// Client usage - parameters match exactly
await chatRoom.sendMessage("user-123", "Hello!", );
```

### Error Handling

Handle action errors appropriately:

```typescript
try  catch (error)  else 
}
```

## Real-time Connections

Real-time connections enable bidirectional communication between clients and actors through persistent connections. Rivet automatically negotiates between WebSocket (preferred for full duplex) and Server-Sent Events (SSE) as a fallback for restrictive environments.

### `connect(params?)` - Establish Stateful Connection

For real-time communication with events, use `.connect()`:

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

### Events

#### `on(eventName, callback)` - Listen for Events

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

#### `once(eventName, callback)` - Listen Once

Listen for an event only once:

```typescript
// Wait for game to start
connection.once("gameStarted", () => );
```

#### `off(eventName, callback?)` - Stop Listening

Remove event listeners:

```typescript
const messageHandler = (message) => console.log(message);

// Add listener
connection.on("messageReceived", messageHandler);

// Remove specific listener
connection.off("messageReceived", messageHandler);

// Remove all listeners for event
connection.off("messageReceived");
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

### Transports

Connections automatically negotiate the best available transport:

#### WebSocket Transport
- **Full duplex**: Client can send and receive
- **Low latency**: Immediate bidirectional communication
- **Preferred**: Used when available

#### Server-Sent Events (SSE)
- **Server-to-client**: Events only, actions via HTTP
- **Fallback**: Used when WebSocket unavailable
- **Compatibility**: Works in restrictive environments

### Reconnections

Connections automatically handle network failures with robust reconnection logic:

**Automatic Behavior:**
- **Exponential backoff**: Retry delays increase progressively to avoid overwhelming the server
- **Action queuing**: Actions called while disconnected are queued and sent once reconnected
- **Event resubscription**: Event listeners are automatically restored on reconnection
- **State synchronization**: Connection state is preserved and synchronized after reconnection

## Authentication

### Connection Parameters

Pass authentication data when connecting to actors:

```typescript
// With connection parameters
const chat = client.chatRoom.getOrCreate(["general"]);
const connection = chat.connect();

// Parameters available in actor via onAuth hook
// Or for action calls
const result = await chat.sendMessage("Hello world!", );
```

### onAuth Hook Validation

Actors can validate authentication using the `onAuth` hook:

```typescript
const protectedActor = actor( = opts;
    
    // Extract token from params or headers
    const token = params.authToken || req.headers.get("Authorization");
    
    if (!token) 
    
    // Validate and return user data
    const user = await validateJWT(token);
    return ;
  },
  
  actions:  = c.conn.auth;
      
      if (role !== "admin") 
      
      return `Hello admin $`;
    }
  }
});
```

Learn more about [authentication patterns](/docs/general/authentication).

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

### Actions vs Connections

**Use Stateless Actions For:**
- Simple request-response operations
- One-off operations  
- Server-side integration
- Minimal overhead required

```typescript
// Good for simple operations
const result = await counter.increment(1);
const status = await server.getStatus();
```

**Use Stateful Connections For:**
- Real-time updates needed
- Multiple related operations
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

// Automatic cleanup with lifecycle
connection.on("disconnected", () => );
```

### Error Handling

Implement proper error handling for both actions and connections:

```typescript
// Action error handling
try  catch (error)  else if (error.code === "UNAUTHORIZED")  else 
}

// Connection error handling
connection.on("error", (error) => );
```

### Performance Optimization

Use appropriate patterns for optimal performance:

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