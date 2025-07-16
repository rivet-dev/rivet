# Connections

Connections represent client connections to your actor. They provide a way to handle client authentication, manage connection-specific data, and control the connection lifecycle.

## Parameters

When clients connect to a actor, they can pass connection parameters that are handled during the connection process.

For example:

```typescript actor.ts
const gameRoom = actor(,
  
  // Handle connection setup
  createConnState: (c, ) => 
    
    // Create connection state
    return ;
  },
  
  actions: 
});
```

```typescript client.ts
const client = createClient("http://localhost:8080");
const gameRoom = await client.gameRoom.get(
});
```

## Connection State

There are two ways to define a actor's connection state:

		Define connection state as a constant value:

		```typescript
		const chatRoom = actor(,
		  
		  // Define default connection state as a constant
		  connState: ,
		  
		  onConnect: (c) => ,
		  
		  actions: 
		});
		```

		Create connection state dynamically with a function. The data returned is used as the initial state of the connection. The connection state can be accessed through `conn.state`.

		```typescript
		const chatRoom = actor(,
		  
		  // Create connection state dynamically
		  createConnState: (c) => ;
		  },
		  
		  actions: );
		      c.broadcast("newMessage", );
		    }
		  }
		});
		```

## Lifecycle Hooks

The connection lifecycle has several hooks:

- `onBeforeConnect`: Called before a client connects, returns the connection state
- `onConnect`: Called when a client successfully connects
- `onDisconnect`: Called when a client disconnects

See the documentation on [Actor Lifecycle](/docs/actors/lifecycle) for more details.

## Connection List

All active connections can be accessed through the context object's `conns` property. This is an array of all current connections.

This is frequently used with `conn.send(name, event)` to send messages directly to clients.

For example:

```typescript
const chatRoom = actor( },
  
  actions: );
      }
    }
  }
});
```

## Disconnecting clients

Connections can be disconnected from within an action:

```typescript
const secureRoom = actor(,
  
  actions: 
    }
  }
});
```

If you need to wait for the disconnection to complete, you can use `await`:

```typescript
await c.conn.disconnect('Too many requests');
```

This ensures the underlying network connections close cleanly before continuing.

## Offline & Auto-Reconnection

See [client documentation](/docs/actors/communicating-with-actors) for details on reconnection behavior.