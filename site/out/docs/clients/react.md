# React

Learn how to create real-time, stateful React applications with Rivet's actor model. The React integration provides intuitive hooks for managing actor connections and real-time updates.

## Installation

Install the Rivet React package:

```bash
npm install @rivetkit/actor @rivetkit/react
```

## Basic Usage

First, set up your actor registry (typically in your backend):

```typescript }
const counter = actor(,  // Skip authentication (can be configured later)
  state: ,
  actions: ,
    getCount: (c) => c.state.count,
  },
});

const registry = setup(,
});
```

Start a server to run your actors:

```typescript }
// Run server with default configuration (port 8080)
registry.runServer();
```

Create a typed client and Rivet hooks:

```tsx }
const client = createClient("http://localhost:8080");
const  = createRivetKit(client);
```

Connect to actors and listen for real-time updates:

```tsx }
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
  - `createWithInput`: Optional input to pass to the actor on creation
  - `createInRegion`: Optional region to create the actor in if does not exist
  - `enabled`: Optional boolean to conditionally enable/disable the connection (default: true)
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

### Authentication

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

Learn more about [authentication](/docs/actors/authentication).