# Cloudflare Workers

Deploy Rivet Actors to Cloudflare Workers with Durable Objects for global edge computing with persistent state.

## Feature Support

| Feature | Supported |
| --- | --- |
| Horizontal scaling | Yes |
| WebSockets | Yes |
| SSE | Yes |
| Edge | Yes |
| Scheduling | Yes |

## Setup

Install the Cloudflare Workers driver:

```bash
npm install @rivetkit/cloudflare-workers
```

Update your server code to support Cloudflare Workers:

```typescript }
const  = createServer(registry);

// Setup router
const app = new Hono();

// Example API endpoint
app.post("/increment/:name", async (c) => );
});

const  = createHandler(app);

;
```

```typescript }
const  = createServerHandler(registry);
;
```

Update your `wrangler.json` configuration to support `ACTOR_DO` and `ACTOR_KV` bindings:

```json }

  ],
  "durable_objects": 
    ]
  },
  "kv_namespaces": [
    
  ]
}
```

**Configuration Requirements:**

- `ACTOR_DO` - Durable Object binding for actor persistence
- `ACTOR_KV` - KV namespace binding for metadata storage
- `nodejs_compat` - Required compatibility flag
- Migration with `ActorHandler` class definition

Deploy your application to Cloudflare Workers:

```bash
wrangler deploy
```

Your actors will now run on Cloudflare's global edge network with persistent state backed by Durable Objects.

## Examples

Example using Cloudflare Workers with Hono web framework.

Basic Cloudflare Workers setup and configuration example.