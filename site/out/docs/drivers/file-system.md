# File System

The File System driver is the default driver for Rivet Actors, providing local file-based storage for state management and inter-actor communication. If no driver is specified in your configuration, the File System driver will be used automatically.

The File System driver is ideal for development and single-node deployments. For production applications that need to scale horizontally across multiple machines, use the [Redis driver](/docs/drivers/redis).

## Feature Support

| Feature | Supported |
| --- | --- |
| Horizontal scaling | No |
| WebSockets | Yes |
| SSE | Yes |
| Edge | No |
| Scheduling | Yes |

## Setup

The File System driver is included with `@rivetkit/actor` - no additional packages needed:

```bash
npm install @rivetkit/actor
```

Use the default configuration with automatic path based on your current working directory:

```typescript }
const driver = createFileSystemDriver();
const  = registry.runServer();

// ...rest of your server...
```

The default path is stored in your system's data path. The path is unique to the current working directory, so you can safely run multiple projects on the same machine.

Specify a custom path for actor storage:

```typescript }
const driver = createFileSystemDriver();
const  = registry.runServer();

// ...rest of your server...
```

**Configuration Options:**

- `path` - Custom directory path for storing actor data (optional)

## Data Management

The path where your actors are stored is printed when you start your project. To reset your actors, delete the folder that is printed.

If running on a single node, make sure to back up your actors folder regularly. `rsync` works nicely with this because each actor is stored as its own file.

## Examples

Basic File System driver setup and configuration example.