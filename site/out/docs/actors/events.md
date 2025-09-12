# Events

Events enable real-time communication from actors to clients. While clients use actions to send data to actors, events allow actors to push updates to connected clients instantly.

Events work through persistent connections such as WebSocket or SSE.

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
  }
});
```

### Sending to Specific Connections

Send events to individual connections using `conn.send(eventName, data)`:

```typescript
const gameRoom = actor( as Record
  },
  
  createConnState: (c, ) => (),
  
  actions: );
      } else 
    }
  }
});
```

Send events to all connections except the sender:

```typescript
const gameRoom = actor( as Record
  },
  
  createConnState: (c, ) => (),
  
  actions: ) => );
          }
        }
      }
    }
  }
});
```

## Subscribing to Events from Clients

Clients must establish a connection to receive events from actors. Use `.connect()` to create a persistent connection, then listen for events.

### Basic Event Subscription

Use `connection.on(eventName, callback)` to listen for events:

```typescript }
const client = createClient("http://localhost:8080");

// Get actor handle and establish connection
const chatRoom = client.chatRoom.getOrCreate(["general"]);
const connection = chatRoom.connect();

// Listen for events
connection.on('messageReceived', (message) => : $`);
  displayMessage(message);
});

// Call actions through the connection
await connection.sendMessage("user-123", "Hello everyone!");
```

```tsx }
function ChatRoom() );

  // Listen for events
  chatRoom.useEvent("messageReceived", (message) => );

  // ...rest of component...
}
```

### One-time Event Listeners

Use `connection.once(eventName, callback)` for events that should only trigger once:

```typescript }
const gameRoom = client.gameRoom.getOrCreate(["room-456"]);
const connection = gameRoom.connect();

// Listen for game start (only once)
connection.once('gameStarted', () => );
```

```tsx }
function GameLobby() 
  });

  // Listen for game start (only once)
  useEffect(() => ;

    gameRoom.connection.once('gameStarted', handleGameStart);
  }, [gameRoom.connection]);

  // ...rest of component...
}
```

### Removing Event Listeners

Use the callback returned from `.on()` to remove event listeners:

```typescript }
// Add listener
const unsubscribe = connection.on('messageReceived', (message) => );

// Remove listener
unsubscribe();
```

```tsx }
function ConditionalListener() );

  useEffect(() => : $`]);
    });

    // Cleanup - remove listener when component unmounts or listening stops
    return () => ;
  }, [chatRoom.connection, isListening]);

  // ...rest of component...
}
```

## More About Connections

For more details on actor connections, including connection lifecycle, authentication, and advanced connection patterns, see the [Connections documentation](/docs/actors/connections).