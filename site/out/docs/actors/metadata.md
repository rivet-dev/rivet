# Metadata

Metadata provides information about the currently running actor.

## Actor ID

Get the unique instance ID of the actor:

```typescript
const actorId = c.actorId;
```

## Actor Name

Get the actor type name:

```typescript
const actorName = c.name;
```

This is useful when you need to know which actor type is running, especially if you have generic utility functions that are shared between different actor implementations.

## Actor Key

Get the actor key used to identify this actor instance:

```typescript
const actorKey = c.key;
```

The key is used to route requests to the correct actor instance and can include parameters passed when creating the actor.

Learn more about using keys for actor addressing and configuration in the [keys documentation](/docs/actors/keys).

## Region

Region can be accessed from the context object via `c.region`.

```typescript
const region = c.region;
```

`c.region` is only supported on Rivet at the moment.

## Example Usage

```typescript }
const chatRoom = actor(,
  
  actions: ;
    }
  }
});

const registry = setup(
});
```

```typescript }
const client = createClient("http://localhost:8080");

// Connect to a chat room
const chatRoom = await client.chatRoom.get("general");

// Get actor metadata
const metadata = await chatRoom.getMetadata();
console.log("Actor metadata:", metadata);
```