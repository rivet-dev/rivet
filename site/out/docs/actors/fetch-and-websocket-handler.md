# Fetch and WebSocket Handler

Actors can handle HTTP requests and WebSocket connections through the `onFetch` and `onWebSocket` handlers.

For most use cases, [actions](/docs/actors/actions) and [events](/docs/actors/events) provide high-level connection handling that's easier to work with. However, raw handlers are required when implementing custom use cases or integrating external libraries that need direct access to the underlying HTTP `Request`/`Response` objects or WebSocket connections.

## Defining Handlers

### `onFetch(c, request, )`

The `onFetch` handler processes HTTP requests sent to your actor. It receives the actor context and a standard `Request` object.

WebSocket upgrades are not currently supported in `onFetch`. Use `onWebSocket` instead.

```typescript
const httpActor = actor(,
    actions: ,
    onFetch(ctx, request) ), ,
            });
        }

        if (url.pathname === "/api/echo" && request.method === "POST") );
        }

        // Return 404 for unhandled paths
        return new Response("Not Found", );
    },
});
```

```typescript
function buildRouter(ctx: ActorContext) );
    });

    app.post("/api/echo", async (c) => );

    app.get("/api/stats", (c) => );
    });

    return app;
}

const honoActor = actor(,
    createVars(ctx) ;
    },
    actions: ,
    async onFetch(ctx, request) ,
});
```

Also see the [raw fetch handler example project](https://github.com/rivet-gg/rivetkit/tree/main/examples/raw-fetch-handler).

	`onFetch` can be used to expose Server-Sent Events from Rivet Actors.

### `onWebSocket(c, websocket, )`

The `onWebSocket` handler manages WebSocket connections. It receives the actor context, a `WebSocket` object, and the initial `Request`.

```typescript
const websocketActor = actor(,
    actions: ,
    onWebSocket(ctx, websocket) ));

        websocket.addEventListener("message", (event) => );
    },
});
```

Also see the [raw WebSocket handler with proxy example project](https://github.com/rivet-gg/rivetkit/tree/main/examples/raw-websocket-handler-proxy).

	Connection lifecycle hooks like `onConnect` and `onDisconnect` do not get called when opening WebSockets for `onWebSocket`. This is because `onWebSocket` provides a low-level connection. Use `ws.addEventListener("open")` and `ws.addEventListener("close")` instead.

## Accessing Your Handlers

There are three ways to access your actor's fetch and WebSocket handlers:

### Option A: From Backend via RivetKit Client

You can use the RivetKit client's built-in methods for raw HTTP and WebSocket access:

```typescript
const client = createClient("http://localhost:8080");

// HTTP requests using .fetch() method
const actor = client.myActor.getOrCreate(["key"]);
const response = await actor.fetch("/api/hello", );
const data = await response.json();

// POST request with JSON body
const postResponse = await actor.fetch("/api/echo", ,
    body: JSON.stringify(),
});
```

```typescript
const client = createClient("http://localhost:8080");

// WebSocket connections using .websocket() method
const actor = client.myActor.getOrCreate(["key"]);
const ws = await actor.websocket("/custom/path");

// Listen for messages
ws.addEventListener("message", (event) =>  else if (message.type === "echo")  else if (message.type === "pong") 
});

// Send messages
ws.send(JSON.stringify());

// Send ping
ws.send(JSON.stringify());
```

For more advanced use cases, you can forward requests to actor handlers from your server:

```typescript
const  = registry.createServer();

const app = new Hono();

// Forward requests to actor's fetch handler
app.all("/forward/:name/*", async (c) => `, "");
    const url = new URL(truncatedPath, c.req.url);
    const newRequest = new Request(url, c.req.raw);
    
    // Forward to actor's fetch handler
    const actor = client.counter.getOrCreate(name);
    const response = await actor.fetch(truncatedPath, newRequest);
    
    return response;
});

serve(app);
```

```typescript
const  = registry.createServer();

const app = new Hono();

// Forward WebSocket connections to actor's WebSocket handler
app.get("/ws/:name", upgradeWebSocket(async (c) => );

            actorWs.addEventListener("close", () => );
        },
        onMessage: (evt, ws) => ,
        onClose: (evt, ws) => ,
    };
}));

serve(app);
```

### Option B: From Frontend with RivetKit Client

Use the RivetKit client to make direct HTTP requests or WebSocket connections:

```typescript
const client = createClient("http://localhost:8080");

// HTTP requests
const actor = client.myActor.getOrCreate(["key"]);
const response = await actor.fetch("/api/hello", );
const data = await response.json();
console.log(data); // 

// POST request with data
const postResponse = await actor.fetch("/api/echo", ,
    body: JSON.stringify(),
});

// Handle response
if (postResponse.ok)  else 
```

```typescript
const client = createClient("http://localhost:8080");

// WebSocket connections
const actor = client.myActor.getOrCreate(["key"]);
const ws = await actor.websocket("/");

// Listen for messages
ws.addEventListener("message", (event) => );
            break;
            
        case "echo":
            console.log("Echo received:", message.data);
            displayMessage(message.data);
            break;
            
        case "pong":
            console.log("Pong received, latency:", Date.now() - message.timestamp);
            break;
            
        case "error":
            console.error("Error:", message.message);
            break;
    }
});

// Send messages
function sendMessage(text) ));
}

// Ping for latency testing
function ping() ));
}

// Handle connection errors
ws.addEventListener("error", (event) => );

ws.addEventListener("close", () => );
```

	The `.websocket()` method returns a barebones WebSocket. Unlike [actor connections](/docs/actors/connections), it does not provide automatic reconnection logic. You must implement reconnection logic yourself if needed.

### Option C: From Frontend via Direct RivetKit Router Access

You can access your actor handlers directly through the mounted RivetKit router. The router automatically handles the required headers for authentication and routing.

For HTTP requests, the router expects these headers:
- `X-RivetKit-Actor-Query`: JSON-encoded actor query
- `X-RivetKit-Encoding`: Encoding type (usually "json")
- `X-RivetKit-Conn-Params`: JSON-encoded connection parameters (optional)

```typescript
// Direct HTTP request to actor
const response = await fetch("http://localhost:8080/registry/actors/myActor/raw/http/api/hello", 
        }),
        "X-RivetKit-Encoding": "json",
        "X-RivetKit-Conn-Params": JSON.stringify()
    }
});

const data = await response.json();
console.log(data); // 

// POST request with data
const postResponse = await fetch("http://localhost:8080/registry/actors/myActor/raw/http/api/echo", 
        }),
        "X-RivetKit-Encoding": "json",
        "X-RivetKit-Conn-Params": JSON.stringify(),
        "Content-Type": "application/json"
    },
    body: JSON.stringify()
});

// Handle response
if (postResponse.ok)  else 
```

For WebSocket connections, authentication data is passed via WebSocket subprotocols:

```typescript
// Direct WebSocket connection to actor
const protocols = [
    `query.$
    }))}`,
    `encoding.json`,
    `conn_params.$))}`
];

const ws = new WebSocket("ws://localhost:8080/registry/actors/myActor/ws/", protocols);

// Listen for messages
ws.addEventListener("message", (event) => );
            
            // Send initial message
            ws.send(JSON.stringify());
            break;
            
        case "echo":
            console.log("Echo received:", message.data);
            break;
            
        case "pong":
            console.log("Pong received, latency:", Date.now() - message.timestamp);
            break;
            
        case "error":
            console.error("WebSocket error:", message.message);
            break;
    }
});

// Send ping for latency testing
function sendPing() ));
}

// Handle connection events
ws.addEventListener("open", () => );

ws.addEventListener("error", (event) => );

ws.addEventListener("close", (event) => );
```

	For Cloudflare Workers, you must include `"rivetkit"` as a protocol when using raw WebSockets:
	
	```typescript
	const protocols = [
	    "rivetkit", // Required for Cloudflare Workers
	    `query.$
	    }))}`,
	    `encoding.json`
	];
	```

## Authentication

If you are using the external client, authentication is handled through the `onAuth` handler. The `onAuth` handler is executed on the server before the request is sent to the actor, reducing resource load on the actor by filtering out unauthorized requests early.

If you are using the server-side client, then authentication is skipped by default.

See the [authentication documentation](/docs/actors/authentication) for detailed information on implementing authentication patterns.

## State Saves

State changes in `onFetch` and `onWebSocket` handlers are automatically saved after the handler finishes executing.

For `onWebSocket` handlers specifically, you'll need to manually save state using `c.saveState()` while the WebSocket connection is open if you want state changes to be persisted immediately. This is because WebSocket connections can remain open for extended periods, and state changes made during event handlers (like `message` events) won't be automatically saved until the connection closes.

```typescript
const websocketActor = actor(,
    actions: ,
    onWebSocket(ctx, websocket) ));
        });
    },
});
```

For more details on state management, see [State](/docs/actors/state).

## W3C Compliance

It's not possible to use the global `fetch` method or global WebSocket class to connect to an actor. This is because actors do not have traditional network interfaces to communicate with.

However, the `Request`, `Response`, and `WebSocket` types used with `.fetch()` and `.websocket()` comply with the W3C specification and will work wherever you pass them.