# Memory

The Memory driver stores all actor state and communication in memory, making it ideal for testing, development, and prototyping scenarios where persistence is not required.

The Memory driver does not persist data between server restarts. For production applications that need to scale horizontally across multiple machines, use the [Redis driver](/docs/drivers/redis).

## Feature Support

| Feature | Supported |
| --- | --- |
| Horizontal scaling | No |
| WebSockets | Yes |
| SSE | Yes |
| Edge | No |
| Scheduling | Yes |

## Setup

The Memory driver is included with `@rivetkit/actor` - no additional packages needed:

```bash
npm install @rivetkit/actor
```

Create and use the Memory driver:

```typescript }
const driver = createMemoryDriver();
const  = registry.runServer();

// ...rest of your server...
```

The Memory driver requires no configuration options.