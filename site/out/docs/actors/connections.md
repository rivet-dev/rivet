# Connections

Connections represent client connections to your actor. They provide a way to handle client authentication, manage connection-specific data, and control the connection lifecycle.

## Parameters

When clients connect to an actor, they can pass connection parameters that are handled during the connection process.

For example:

```typescript }
const client = createClient("http://localhost:8080");
const gameRoom = await client.gameRoom.get(
});
```

```typescript }
const gameRoom = actor(,
  
  // Handle connection setup
  createConnState: (c, ) => 
    
    // Create connection state
    return ;
  },
  
  actions: 
});
```

## Connection State

There are two ways to define an actor's connection state:

		Define connection state as a constant value:

		```typescript
		const chatRoom = actor(,
		  
		  // Define default connection state as a constant
		  connState: ,
		  
		  onConnect: (c) => ,
		  
		  actions: 
		});
		```

		This value will be cloned for every new connection using `structuredClone`.

		Create connection state dynamically with a function called for each connection:

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

## Connection Lifecycle Hooks

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

## Connection Error Handling

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

## Offline & Auto-Reconnection

See [client documentation](/docs/actors/communicating-between-actors) for details on reconnection behavior.