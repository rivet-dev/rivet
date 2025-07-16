# Metadata

Metadata provides information about the currently running actor.

## Region

Region can be accessed from the context object via `c.region`.

`c.region` is only supported on Rivet at the moment.

## Tags

Tags can be accessed from the context object via `c.tags`.

For example:

```typescript chat_room.ts
const chatRoom = actor(,
  
  actions: ,
    
    addMessage: (c, message) => `);
      
      c.state.messages.push();
      
      c.broadcast('newMessage', );
    }
  }
});

default chatRoom;
```

```typescript client.ts
const client = createClient("http://localhost:8080");

// Connect to a specific channel
const randomChannel = await client.chatRoom.get();

// Check the channel ID
const channelId = await randomChannel.getChannelId();
console.log("Connected to channel:", channelId); // "random"

// Or connect with multiple parameters
const teamChannel = await client.chatRoom.get();
```

## Actor Name

You can access the actor name with:

```typescript
const actorName = c.name;
```

This is useful when you need to know which actor type is running, especially if you have generic utility functions that are shared between different actor implementations.