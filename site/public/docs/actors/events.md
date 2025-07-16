# Events

Real-time communication between actors and clients

Events enable real-time communication from actors to clients. While clients use actions to send data to actors, events allow actors to push updates to connected clients instantly.

Events work through persistent connections (WebSocket or SSE). Clients establish connections using `.connect()` and then listen for events with `.on()`.

## Publishing Events from Actors

### Broadcasting to All Clients

Use `c.broadcast(eventName, data)` to send events to all connected clients:

```typescript
const chatRoom = actor(,
  
  actions: ;
      
      c.state.messages.push(message);
      
      // Broadcast to all connected clients
      c.broadcast('messageReceived', message);
      
      return message;
    },
    
    deleteMessage: (c, messageId: string) => );
      }
    }
  }
});
```

### Sending to Specific Connections

Send events to individual connections using `conn.send(eventName, data)`:

```typescript
const gameRoom = actor( as Record
  },
  
  createConnState: (c, ) => (),
  
  actions: ) => );
          }
        }
      }
    },
    
    sendPrivateMessage: (c, targetPlayerId: string, message: string) => );
      } else 
    }
  }
});
```

### Event Filtering by Connection State

Filter events based on connection properties:

```typescript
const newsRoom = actor(,
  
  createConnState: (c, ) => (),
  
  actions: ) => ;
      
      c.state.articles.push(newArticle);
      
      // Send to appropriate subscribers only
      for (const conn of c.conns) 
      }
      
      return newArticle;
    }
  }
});
```

## Subscribing to Events from Clients

Clients must establish a connection to receive events from actors. Use `.connect()` to create a persistent connection, then listen for events.

### Basic Event Subscription

Use `connection.on(eventName, callback)` to listen for events:

```typescript
const client = createClient("http://localhost:8080");

// Get actor handle and establish connection
const chatRoom = client.chatRoom.getOrCreate(["general"]);
const connection = chatRoom.connect();

// Listen for events
connection.on('messageReceived', (message) => : $`);
  displayMessage(message);
});

connection.on('messageDeleted', () =>  was deleted`);
  removeMessageFromUI(messageId);
});

// Call actions through the connection
await connection.sendMessage("user-123", "Hello everyone!");
```

### One-time Event Listeners

Use `connection.once(eventName, callback)` for events that should only trigger once:

```typescript
const gameRoom = client.gameRoom.getOrCreate(["room-456"]);
const connection = gameRoom.connect();

// Listen for game start (only once)
connection.once('gameStarted', () => );

// Listen for game events continuously
connection.on('playerMoved', () => );

connection.on('privateMessage', () => );
```

### Removing Event Listeners

Use `connection.off()` to remove event listeners:

```typescript
const messageHandler = (message) => ;

// Add listener
connection.on('messageReceived', messageHandler);

// Remove specific listener
connection.off('messageReceived', messageHandler);

// Remove all listeners for an event
connection.off('messageReceived');

// Remove all listeners
connection.off();
```

### React Integration

Rivet's React hooks provide a convenient way to handle events in React components:

```tsx
function ChatRoom() );

  // Listen for new messages
  chatRoom.useEvent("messageReceived", (message) => );

  // Listen for deleted messages
  chatRoom.useEvent("messageDeleted", () => );

  const sendMessage = async (text: string) => ;

  return (
    
      : 
        
      ))}

  );
}
```

## Connection Lifecycle Events

Connections emit lifecycle events you can listen to:

```typescript
const connection = actor.connect();

connection.on('connected', () => );

connection.on('disconnected', () => );

connection.on('reconnected', () => );

connection.on('error', (error) => );
```

## Advanced Event Patterns

### Event Buffering

Events are automatically buffered during disconnections and replayed on reconnection:

```typescript
const connection = actor.connect();

// Events sent while disconnected are queued
connection.on('importantUpdate', (data) => );
```

### Connection Parameters

Pass parameters when connecting to provide context to the actor:

```typescript
const gameRoom = client.gameRoom.getOrCreate(["competitive-room"]);
const connection = gameRoom.connect();

// The actor can use these parameters in its onBeforeConnect hook
// or access them via c.conn.params in actions
```

### Conditional Event Handling

Handle events conditionally based on connection state:

```typescript
connection.on('playerMoved', () => 
});

connection.on('newArticle', (article) =>  else 
});
```

## Error Handling

Handle event-related errors gracefully:

```typescript
try  catch (error) 
  });
  
} catch (error) 
```

## Best Practices

1. **Always use connections for events**: Events only work through `.connect()`, not direct action calls
2. **Handle connection lifecycle**: Listen for connection, disconnection, and error events
3. **Clean up listeners**: Remove event listeners when components unmount
4. **Validate event data**: Don't assume event payloads are always correctly formatted
5. **Use React hooks**: For React apps, use `useActor` and `actor.useEvent` for automatic cleanup
6. **Buffer critical events**: Design actors to resend important events on reconnection if needed