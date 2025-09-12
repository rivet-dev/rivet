# Redis

The Redis driver enables deploying scalable Rivet Actors using Redis as the backend for state management and inter-actor communication.

The Redis driver is currently in preview. We do not recommend shipping production applications with the Redis driver yet.

If you want to take Redis to production, [contact us](/support) so we can help validate your setup is production ready and help resolve issues promptly.

## Feature Support

| Feature | Supported |
| --- | --- |
| Horizontal scaling | Yes |
| WebSockets | Yes |
| SSE | Yes |
| Edge | No |
| Scheduling | [Not yet](https://github.com/rivet-gg/rivetkit/issues/1095) |

## Setup

Install the required packages:

```bash
npm install @rivetkit/redis ioredis@5
```

Configure your application using environment variables:

```bash
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=your-password
REDIS_KEY_PREFIX=myproject
```

**Available Environment Variables:**

- `REDIS_HOST` - Redis server hostname (default: `localhost`)
- `REDIS_PORT` - Redis server port (default: `6379`)
- `REDIS_PASSWORD` - Redis password (optional)
- `REDIS_KEY_PREFIX` - Key prefix for isolation when running multiple projects (optional)

Then start your server:

```typescript }
const driver = createRedisDriver();
const  = registry.runServer();

// ...rest of your server...
```

For advanced configuration, pass your own Redis instance:

```typescript }
const redis = new Redis();

const driver = createRedisDriver();
const  = registry.runServer();

// ...rest of your server...
```

**Configuration Options:**

When passing a custom Redis instance, you have full control over the connection options. Common options include:

- `host` - Redis server hostname
- `port` - Redis server port
- `password` - Redis password

See the [ioredis documentation](https://github.com/luin/ioredis) for all available options.

To prevent data loss, ensure AOF (Append Only File) persistence is enabled on your Redis server. See the [Redis Persistence Documentation](https://redis.io/docs/latest/operate/oss_and_stack/management/persistence/#append-only-file) for setup instructions.

## Deploy

Deploy your Redis-powered actors on these hosting providers:

Deploy on Railway with automatic scaling and managed infrastructure.

## Examples

Example using Redis driver with Hono web framework.

Basic Redis driver setup and configuration example.