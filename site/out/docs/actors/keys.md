# Actor Keys

Actor keys uniquely identify actor instances within each actor type. Keys are used for addressing which specific actor to communicate with.

## Key Format

Actor keys can be either a string or an array of strings:

```typescript
// String key
const counter = client.counter.getOrCreate("my-counter");

// Array key (compound key)
const chatRoom = client.chatRoom.getOrCreate(["room", "general"]);
```

### Compound Keys & User Data

Array keys are useful when you need compound keys with user-provided data. Using arrays makes adding user data safe by preventing key injection attacks:

```typescript
// User-specific chat rooms
const userRoom = client.chatRoom.getOrCreate(["user", userId, "private"]);

// Game rooms by region and difficulty
const gameRoom = client.gameRoom.getOrCreate(["us-west", "hard", gameId]);

// Multi-tenant resources
const workspace = client.workspace.getOrCreate(["tenant", tenantId, workspaceId]);
```

This allows you to create hierarchical addressing schemes and organize actors by multiple dimensions.

Don't build keys using string interpolation like `"foo:$:bar"` when `userId` contains user data. If a user provides a value containing the delimiter (`:` in this example), it can break your key structure and cause key injection attacks.

### Omitting Keys

You can create actors without specifying a key in situations where there is a singleton actor (i.e. only one actor of a given type). For example:

```typescript
// Get the singleton session
const globalActor = client.globalActor.getOrCreate();
```

This pattern should be avoided, since a singleton actor usually means you have a single actor serving all traffic & your application will not scale. See [scaling documentation](/docs/actors/scaling) for more information.

### Key Uniqueness

Keys are unique within each actor name. Different actor types can use the same key:

```typescript
// These are different actors, same key is fine
const userChat = client.chatRoom.getOrCreate(["user-123"]);
const userProfile = client.userProfile.getOrCreate(["user-123"]);
```

## Accessing Keys in Metadata

Access the actor's key within the actor using the [metadata](/docs/actors/metadata) API:

```typescript }
const chatRoom = actor(
  }
});

const registry = setup(
});
```

```typescript }
const client = createClient("http://localhost:8080");

async function connectToRoom(roomName: string) 

// Usage example
const generalRoom = await connectToRoom("general");
```

## Configuration Examples

### Simple Configuration with Keys

Use keys to provide basic actor configuration:

```typescript }
const userSession = actor(
  }),
  
  actions: 
});

const registry = setup(
});
```

```typescript }
const client = createClient("http://localhost:8080");

// Pass user ID in the key for user-specific actors
const userSession = client.userSession.getOrCreate([userId]);
```

### Complex Configuration with Input

For more complex configuration, use [input parameters](/docs/actors/input):

```typescript }
const client = createClient("http://localhost:8080");

// Create with both key and input
const chatRoom = await client.chatRoom.create(["room", roomName], 
  }
});
```