# Registry

Configure and manage your actor registry

The registry is the central configuration hub for your Rivet application. It defines which actors are available and how your application runs.

## Basic Setup

Create a registry by importing your actors and using the `setup` function:

```typescript
const registry = setup(,
});
```

## Creating Servers

### Development Server

For development, create and run a server directly:

```typescript
// Start a development server
registry.runServer(,
    manager: ,
  },
});
```

### Production Setup

For production, get the handler and integrate with your framework:

```typescript
// Create server components
const  = registry.createServer(,
    manager: ,
  },
});

// Use with Hono
const app = new Hono();
app.route("/registry", hono);

// Or use the handler directly
app.all("/registry/*", handler);

// Start the server
serve(app);
```

## Configuration Options

### Driver Configuration

The driver configuration determines how actors are stored and managed:

```typescript
const  = registry.createServer(,
    
    // Manager coordination
    manager: ,
  },
});
```

### Topology Options

- **`standalone`**: Single process, good for development
- **`partition`**: Distributed actors, good for production scaling
- **`coordinate`**: Peer-to-peer coordination, good for high availability

### Storage Drivers

- **`memory`**: In-memory storage, data lost on restart
- **`file-system`**: Persistent file-based storage
- **`redis`**: Redis-backed persistence and coordination
- **`rivet`**: Rivet platform integration

### CORS Configuration

Configure CORS for browser clients:

```typescript
registry.runServer(,
});
```

### Request Limits

Configure request size limits:

```typescript
registry.runServer();
```

## Worker Mode

For distributed topologies, you can create worker instances:

```typescript
// Manager instance (handles routing)
const  = registry.createServer(,
});

// Worker instance (runs actors)
const  = registry.createWorker(,
});
```

## Type Safety

The registry provides full type safety for your client:

```typescript
// TypeScript knows about your actors
const counter = client.counter.getOrCreate(["my-counter"]);
const chatRoom = client.chatRoom.getOrCreate(["general"]);

// Action calls are type-checked
const count: number = await counter.increment(5);
```

## Testing Configuration

Use memory drivers for testing:

```typescript
// test-registry.ts
const testRegistry = setup(,
});

// In your tests
const  = testRegistry.createServer(,
    manager: ,
  },
});
```

## Environment-Specific Configuration

Use environment variables to configure different environments:

```typescript
const isProd = process.env.NODE_ENV === "production";
const redisUrl = process.env.REDIS_URL || "redis://localhost:6379";

const registry = setup(,
});

// Environment-specific server creation
function createAppServer() ,
          manager: ,
        }
      : ,
          manager: ,
        },
    cors: ,
  });
}
```

## Best Practices

### Registry Organization

Keep your registry clean and organized:

```typescript
// actors/index.ts - Export all actors
 from "./counter";
 from "./chat-room";
 from "./game";

// registry.ts - Import and configure
const registry = setup();
```

### Actor Naming

Use consistent naming conventions:

```typescript
const registry = setup(,
});
```

### Configuration Management

Separate configuration from registry definition:

```typescript
// config.ts
const appConfig = ,
  cors: ,
};

// server.ts
const  = registry.createServer(,
    manager: ,
  },
  cors: appConfig.cors,
});

serve();
```