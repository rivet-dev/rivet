# React

Build real-time React applications with Rivet Actors

Learn how to create real-time, stateful React applications with Rivet's actor model. The React integration provides intuitive hooks for managing actor connections and real-time updates.

## Installation

Install the Rivet React package:

```bash
npm install @rivetkit/actor @rivetkit/react
```

## Basic Usage

First, set up your actor registry (typically in your backend):

```typescript
// backend/registry.ts
const counter = actor(,
  actions: ,
    getCount: (c) => c.state.count,
  },
});

const registry = setup(,
});
```

Create a typed client and Rivet hooks:

```tsx
// src/rivetkit.ts
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);
```

Connect to actors and listen for real-time updates:

```tsx
// src/App.tsx
function App() );

  // Listen for real-time count updates
  counter.useEvent("countChanged", (newCount: number) => );

  const increment = async () => ;

  return (
    
      Rivet Counter
      Count: 

          Counter Name:
           setCounterName(e.target.value)}
            style=}
          />

        Increment

        Status: 

  );
}

default App;
```

## API Reference

### `createRivetKit(client, options?)`

Creates the Rivet hooks for React integration.

```tsx
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);
```

#### Parameters

- `client`: The Rivet client created with `createClient`
- `options`: Optional configuration object

#### Returns

An object containing:
- `useActor`: Hook for connecting to actors

### `useActor(options)`

Hook that connects to an actor and manages the connection lifecycle.

```tsx
const actor = useActor(,
  enabled: true
});
```

#### Parameters

- `options`: Object containing:
  - `name`: The name of the actor type (string)
  - `key`: Array of strings identifying the specific actor instance
  - `params`: Optional parameters passed to the actor connection
  - `enabled`: Optional boolean to conditionally enable/disable the connection (default: true)

#### Returns

Actor object with the following properties:
- `connection`: The actor connection for calling actions, or `null` if not connected
- `isConnected`: Boolean indicating if the actor is connected
- `state`: Current actor state (if available)
- `useEvent(eventName, handler)`: Method to subscribe to actor events

### `actor.useEvent(eventName, handler)`

Subscribe to events emitted by the actor.

```tsx
const actor = useActor();

actor.useEvent("countChanged", (newCount: number) => );
```

#### Parameters

- `eventName`: The name of the event to listen for (string)
- `handler`: Function called when the event is emitted

#### Lifecycle

The event subscription is automatically managed:
- Subscribes when the actor connects
- Cleans up when the component unmounts or actor disconnects
- Re-subscribes on reconnection

## Advanced Patterns

### Multiple Actors

Connect to multiple actors in a single component:

```tsx
function Dashboard() );
  
  const notifications = useActor();

  userProfile.useEvent("profileUpdated", (profile) => );

  notifications.useEvent("newNotification", (notification) => );

  return (

  );
}
```

### Conditional Connections

Control when actors connect using the `enabled` option:

```tsx
function ConditionalActor() );

  return (
    
       setEnabled(!enabled)}>

      )}
    
  );
}
```

### Authentication

Pass authentication parameters to actors:

```tsx
function AuthenticatedChat() 
  });

  chatRoom.useEvent("messageReceived", (message) => );

  const sendMessage = async (text: string) => ;

  return (

  );
}
```

### Error Handling

Handle connection errors gracefully:

```tsx
function ResilientCounter() );

  counter.useEvent("error", (err) => );

  counter.useEvent("connected", () => );

  return (

      )}
      
        Status: 

  );
}
```

### Custom Hooks

Create reusable custom hooks for common patterns:

```tsx
// Custom hook for a counter with persistent state
function useCounter(counterId: string) );

  counter.useEvent("countChanged", setCount);

  const increment = useCallback(async (amount = 1) => , [counter.connection]);

  const reset = useCallback(async () => , [counter.connection]);

  return ;
}

// Usage
function App()  = useCounter("my-counter");

  return (
    
      Count: 
       increment()} disabled=>
        Increment
      
       reset()} disabled=>
        Reset

  );
}
```

### Real-time Collaboration

Build collaborative features with multiple event listeners:

```tsx
function CollaborativeEditor() );
  
  const document = useActor(
  });

  // Listen for content changes
  document.useEvent("contentChanged", (newContent) => );

  // Listen for cursor movements
  document.useEvent("cursorMoved", () => ));
  });

  // Listen for user join/leave
  document.useEvent("userJoined", () =>  joined the document`);
  });

  document.useEvent("userLeft", () =>  = prev;
      return rest;
    });
  });

  const updateContent = async (newContent: string) => ;

  return (

  );
}
```

## Client Connection Options

### Basic Client Setup

Create a type-safe client to connect to your backend:

```ts client.ts
// Create typed client
const client = createClient("http://localhost:8080");

// Use the counter actor directly
const counter = client.counter.getOrCreate(["my-counter"]);

// Call actions
const count = await counter.increment(3);
console.log("New count:", count);

// Get current state
const currentCount = await counter.getCount();
console.log("Current count:", currentCount);

// Listen to real-time events
const connection = counter.connect();
connection.on("countChanged", (newCount) => );

// Increment through connection
await connection.increment(1);
```

### React Integration

Use the React hooks for seamless integration:

```tsx
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);

function App() );

	counter.useEvent("countChanged", (newCount: number) => setCount(newCount));

	const increment = async () => ;

	return (
		
			Counter: 
			 setCounterName(e.target.value)}
				placeholder="Counter name"
			/>
			Increment
		
	);
}
```

## Environment Configuration

### Development vs Production

Create environment-specific configurations:

```ts config.ts
const isDev = process.env.NODE_ENV !== "production";

const config = ,
					manager: ,
				}
			: ,
					manager: ,
				},
	},
};
```

### Backend Configuration

Update your server to use environment-based configuration:

```ts server.ts
const  = registry.createServer(config.rivetkit);

// ... rest of server setup
```

### Frontend Environment Variables

Configure your frontend for different environments:

```ts .env.local
VITE_API_URL=http://localhost:8080
VITE_WS_URL=ws://localhost:8080
```

```ts config/client.ts
const API_URL = import.meta.env.VITE_API_URL || "http://localhost:8080";

const client = createClient(API_URL);
```

## Authentication Integration

### Protected Actors

Add authentication to secure your actors:

```ts registry.ts
const protectedCounter = actor(
		
		// Validate token and return user data
		const user = await validateJWT(token);
		return ;
	},
	
	state: ,
	
	actions:  = c.conn.auth;
			
			c.state.count += amount;
			c.broadcast("countChanged", );
			return c.state.count;
		},
	},
});
```

### React Authentication

Connect authenticated actors in React:

```tsx
function AuthenticatedApp() ,
		enabled: !!authToken // Only connect when authenticated
	});

	const login = async () => ;

	if (!authToken) 

	return (
		
			Authenticated Counter

	);
}
```

Learn more about [authentication](/docs/general/authentication).

## Best Practices

1. **Use Custom Hooks**: Extract actor logic into reusable custom hooks
2. **Handle Loading States**: Always account for the initial loading state
3. **Error Boundaries**: Implement error boundaries around actor components
4. **Conditional Connections**: Use the `enabled` prop to control when actors connect
5. **Event Cleanup**: Event listeners are automatically cleaned up, but be mindful of heavy operations in handlers
6. **State Management**: Combine with React state for local UI state that doesn't need to be shared