---
name: rivetkit-actors
description: Build stateful backends with Rivet Actors: high-performance, long-lived, in-memory, persisted processes. Use when you outgrow HTTP, databases, or queues. Trigger on tasks related to AI agents, multiplayer apps or games, collaborative editors, realtime solutions, workflow automation, geo-distributed or per-tenant databases, local-first sync, WebSocket servers, background or cron jobs, rate limiters, or CRDT docs.
---

# Rivet Actors

Build high-performance, long-lived, in-memory processes on Rivet's actor runtime. Use this skill whenever HTTP endpoints, databases, or queues no longer meet the requirements for AI agents, realtime collaboration, workflow orchestration, or geo-distributed data.

## MCP Server (Recommended)

For the best experience building with Rivet, install the Rivet MCP server. This gives you access to live documentation search (`docs.search`, `docs.get`) so every response is grounded in official Rivet docs.

**Prompt the user:**
> Would you like to install the Rivet MCP server for enhanced documentation access? See [rivet.gg/docs/general/mcp](https://rivet.gg/docs/general/mcp) for setup instructions.

Once installed, always call `docs.search` and `docs.get` before answering Rivet-related questions to ensure responses cite the latest official documentation.

## First Steps

1. Install RivetKit
   ```bash
   npm install rivetkit
   ```
2. Define a registry with `setup({ use: { /* actors */ } })`.
3. Expose `registry.serve()` or `registry.handler()` (serverless) or `registry.startRunner()` (runner mode).
4. Verify `/api/rivet/metadata` returns 200 before deploying.
5. Configure Rivet Cloud or self-hosted engine (registry token, project, region, metadata endpoint).
6. Integrate clients

## Minimal Project

### Backend

**actors.ts**

```ts
import { actor, setup } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      c.broadcast("count", c.state.count);
      return c.state.count;
    },
  },
});

export const registry = setup({
  use: { counter },
});
```

**server.ts**

Integrate with the user's existing server if applicable. Otherwise, default to Hono.

### No Framework

```typescript
import { actor, setup } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c, amount: number) => c.state.count += amount }
});

const registry = setup({ use: { counter } });

// Exposes Rivet API on /api/rivet/ to communicate with actors
export default registry.serve();
```

### Hono

```typescript
import { Hono } from "hono";
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c, amount: number) => c.state.count += amount }
});

const registry = setup({ use: { counter } });

// Build client to communicate with actors (optional)
const client = createClient<typeof registry>();

const app = new Hono();

// Exposes Rivet API to communicate with actors
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
```

### Elysia

```typescript
import { Elysia } from "elysia";
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c, amount: number) => c.state.count += amount }
});

const registry = setup({ use: { counter } });

// Build client to communicate with actors (optional)
const client = createClient<typeof registry>();

const app = new Elysia()
	// Exposes Rivet API to communicate with actors
	.all("/api/rivet/*", (c) => registry.handler(c.request));

export default app;

```

### Minimal Client

```typescript
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c, amount: number) => c.state.count += amount }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>();
const counterHandle = client.counter.getOrCreate(["my-counter"]);
await counterHandle.increment(1);
```

See the [client quick reference](#javascript-client-quick-reference) for more details.

## Actor Quick Reference

### State

Persistent data that survives restarts, crashes, and deployments. State is persisted on Rivet Cloud or Rivet self-hosted, so it survives restarts if the current process crashes or exits.

### Static Initial State

```ts
import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c) => c.state.count += 1,
  },
});
```

### Dynamic Initial State

```ts
import { actor } from "rivetkit";

interface CounterState {
  count: number;
}

const counter = actor({
  state: { count: 0 } as CounterState,
  createState: (c, input: { start?: number }): CounterState => ({
    count: input.start ?? 0,
  }),
  actions: {
    increment: (c) => c.state.count += 1,
  },
});
```

[Documentation](/docs/actors/state)

### Input

Pass initialization data when creating actors.

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const game = actor({
  createState: (c, input: { mode: string }) => ({ mode: input.mode }),
  actions: {},
});

const registry = setup({ use: { game } });
const client = createClient<typeof registry>();

// Client usage
const gameHandle = client.game.getOrCreate(["game-1"], {
  createWithInput: { mode: "ranked" }
});
```

[Documentation](/docs/actors/input)

### Temporary Variables

Temporary data that doesn't survive restarts. Use for non-serializable objects (event emitters, connections, etc).

### Static Initial Vars

```ts
import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  vars: { lastAccess: 0 },
  actions: {
    increment: (c) => {
      c.vars.lastAccess = Date.now();
      return c.state.count += 1;
    },
  },
});
```

### Dynamic Initial Vars

```ts
import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  createVars: () => ({
    emitter: new EventTarget(),
  }),
  actions: {
    increment: (c) => {
      c.vars.emitter.dispatchEvent(new Event("change"));
      return c.state.count += 1;
    },
  },
});
```

[Documentation](/docs/actors/ephemeral-variables)

### Actions

Actions are the primary way clients and other actors communicate with an actor.

```ts
import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => (c.state.count += amount),
    getCount: (c) => c.state.count,
  },
});
```

[Documentation](/docs/actors/actions)

### Events & Broadcasts

Events enable real-time communication from actors to connected clients.

```ts
import { actor } from "rivetkit";

const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {
    sendMessage: (c, text: string) => {
      // Broadcast to ALL connected clients
      c.broadcast("newMessage", { text });
    },
  },
});
```

[Documentation](/docs/actors/events)

### Connections

Access all connected clients via `c.conns`. Each connection has state defined by `connState` or `createConnState`.

```ts
import { actor } from "rivetkit";

interface ConnState {
  userId: string;
}

const chatRoom = actor({
  state: {},
  connState: { userId: "" } as ConnState,
  createConnState: (c, params: { userId: string }): ConnState => ({ userId: params.userId }),
  actions: {
    // Send to a specific connection
    sendPrivate: (c, targetUserId: string, text: string) => {
      for (const conn of c.conns.values()) {
        if (conn.state.userId === targetUserId) {
          conn.send("privateMessage", { text });
          break;
        }
      }
    },
    // Send to all except current connection
    notifyOthers: (c, text: string) => {
      for (const conn of c.conns.values()) {
        if (conn !== c.conn) conn.send("notification", { text });
      }
    },
    // Disconnect a client
    kickUser: (c, userId: string) => {
      for (const conn of c.conns.values()) {
        if (conn.state.userId === userId) {
          conn.disconnect("Kicked by admin");
          break;
        }
      }
    },
  },
});
```

[Documentation](/docs/actors/connections)

### Actor-to-Actor Communication

Actors can call other actors using `c.client()`.

```ts
import { actor, setup } from "rivetkit";

const inventory = actor({
  state: { stock: 100 },
  actions: {
    reserve: (c, amount: number) => { c.state.stock -= amount; }
  }
});

const order = actor({
  state: {},
  actions: {
    process: async (c) => {
      const client = c.client<typeof registry>();
      await client.inventory.getOrCreate(["main"]).reserve(1);
    },
  },
});

const registry = setup({ use: { inventory, order } });
```

[Documentation](/docs/actors/communicating-between-actors)

### Scheduling

Schedule actions to run after a delay or at a specific time. Schedules persist across restarts, upgrades, and crashes.

```ts
import { actor } from "rivetkit";

const reminder = actor({
  state: { message: "" },
  actions: {
    // Schedule action to run after delay (ms)
    setReminder: (c, message: string, delayMs: number) => {
      c.state.message = message;
      c.schedule.after(delayMs, "sendReminder");
    },
    // Schedule action to run at specific timestamp
    setReminderAt: (c, message: string, timestamp: number) => {
      c.state.message = message;
      c.schedule.at(timestamp, "sendReminder");
    },
    sendReminder: (c) => {
      c.broadcast("reminder", { message: c.state.message });
    },
  },
});
```

[Documentation](/docs/actors/schedule)

### Destroying Actors

Permanently delete an actor and its state using `c.destroy()`.

```ts
import { actor } from "rivetkit";

const userAccount = actor({
  state: { email: "", name: "" },
  onDestroy: (c) => {
    console.log(`Account ${c.state.email} deleted`);
  },
  actions: {
    deleteAccount: (c) => {
      c.destroy();
    },
  },
});
```

[Documentation](/docs/actors/destroy)

### Lifecycle Hooks

Actors support hooks for initialization, connections, networking, and state changes.

```ts
import { actor } from "rivetkit";

interface RoomState {
  users: Record<string, boolean>;
  name?: string;
}

interface RoomInput {
  roomName: string;
}

interface ConnState {
  userId: string;
  joinedAt: number;
}

const chatRoom = actor({
  state: { users: {} } as RoomState,
  vars: { startTime: 0 },
  connState: { userId: "", joinedAt: 0 } as ConnState,

  // State & vars initialization
  createState: (c, input: RoomInput): RoomState => ({ users: {}, name: input.roomName }),
  createVars: () => ({ startTime: Date.now() }),

  // Actor lifecycle
  onCreate: (c) => console.log("created", c.key),
  onDestroy: (c) => console.log("destroyed"),
  onWake: (c) => console.log("actor started"),
  onSleep: (c) => console.log("actor sleeping"),
  onStateChange: (c, newState) => c.broadcast("stateChanged", newState),

  // Connection lifecycle
  createConnState: (c, params): ConnState => ({ userId: (params as { userId: string }).userId, joinedAt: Date.now() }),
  onBeforeConnect: (c, params) => { /* validate auth */ },
  onConnect: (c, conn) => console.log("connected:", conn.state.userId),
  onDisconnect: (c, conn) => console.log("disconnected:", conn.state.userId),

  // Networking
  onRequest: (c, req) => new Response(JSON.stringify(c.state)),
  onWebSocket: (c, socket) => socket.addEventListener("message", console.log),

  // Response transformation
  onBeforeActionResponse: <Out>(c: unknown, name: string, args: unknown[], output: Out): Out => output,

  actions: {},
});
```

[Documentation](/docs/actors/lifecycle)

## JavaScript Client Quick Reference

### Stateless vs Stateful

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      c.broadcast("count", c.state.count);
      return c.state.count;
    }
  }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>();
const counterHandle = client.counter.getOrCreate(["my-counter"]);

// Stateless: each call is independent, no persistent connection
await counterHandle.increment(1);

// Stateful: persistent connection for realtime events
const conn = counterHandle.connect();
conn.on("count", (value: number) => console.log(value));
await conn.increment(1);
```

[Documentation](/docs/actors/clients)

### Getting Actors

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {}
});

const game = actor({
  state: { mode: "" },
  createState: (c, input: { mode: string }) => ({ mode: input.mode }),
  actions: {}
});

const registry = setup({ use: { chatRoom, game } });
const client = createClient<typeof registry>();

// Get or create by key
const room = client.chatRoom.getOrCreate(["room-42"]);

// Get existing (returns null if not found)
const existing = client.chatRoom.get(["room-42"]);

// Create with input
const gameHandle = client.game.create(["game-1"], { input: { mode: "ranked" } });
```

[Documentation](/docs/actors/keys)

### Subscribing to Events

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>();

const conn = client.chatRoom.getOrCreate(["general"]).connect();
conn.on("message", (msg: string) => console.log(msg));
conn.once("gameOver", () => console.log("done"));
```

[Documentation](/docs/actors/events)

### Calling from Backend

Call actors from your server-side code.

```ts
import { Hono } from "hono";
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      return c.state.count;
    }
  }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>();
const app = new Hono();

app.post("/increment/:name", async (c) => {
  const counterHandle = client.counter.getOrCreate([c.req.param("name")]);
  const newCount = await counterHandle.increment(1);
  return c.json({ count: newCount });
});
```

[Documentation](/docs/clients/javascript)

## React Quick Reference

### Setup

```typescript {{"title":"registry.ts"}} @hide
import { actor, setup } from "rivetkit";

export const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      c.broadcast("count", c.state.count);
      return c.state.count;
    }
  }
});

export const registry = setup({ use: { counter } });
```

```tsx {{"title":"app.tsx"}}
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

const { useActor } = createRivetKit<typeof registry>();
```

### useActor & Calling Actions

```typescript {{"title":"registry.ts"}} @hide
import { actor, setup } from "rivetkit";

export const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      return c.state.count;
    }
  }
});

export const registry = setup({ use: { counter } });
```

```tsx {{"title":"counter.tsx"}}
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

const { useActor } = createRivetKit<typeof registry>();

function Counter() {
  const counter = useActor({ name: "counter", key: ["my-counter"] });

  const handleClick = async () => {
    await counter.connection?.increment(1);
  };

  return <button onClick={handleClick}>+</button>;
}
```

### Subscribing to Events

```typescript {{"title":"registry.ts"}} @hide
import { actor, setup } from "rivetkit";

export const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {
    send: (c, msg: string) => {
      c.state.messages.push(msg);
      c.broadcast("message", msg);
    }
  }
});

export const registry = setup({ use: { chatRoom } });
```

```tsx {{"title":"chat.tsx"}}
import { useState } from "react";
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

const { useActor } = createRivetKit<typeof registry>();

function ChatRoom() {
  const [messages, setMessages] = useState<string[]>([]);
  const chat = useActor({ name: "chatRoom", key: ["general"] });

  chat.useEvent("message", (msg: string) => setMessages((prev) => [...prev, msg]));

  return <div>{messages.map((m, i) => <p key={i}>{m}</p>)}</div>;
}
```

[Documentation](/docs/clients/react)

## Reference Map

### Actors

- [Actions](reference/actors-actions.md)
- [Actor Keys](reference/actors-keys.md)
- [Actor Scheduling](reference/actors-schedule.md)
- [AI and User-Generated Rivet Actors](reference/actors-ai-and-user-generated-actors.md)
- [Authentication](reference/actors-authentication.md)
- [Clients](reference/actors-clients.md)
- [Cloudflare Workers Quickstart](reference/actors-quickstart-cloudflare-workers.md)
- [Communicating Between Actors](reference/actors-communicating-between-actors.md)
- [Connections](reference/actors-connections.md)
- [Design Patterns](reference/actors-design-patterns.md)
- [Destroying Actors](reference/actors-destroy.md)
- [Ephemeral Variables](reference/actors-ephemeral-variables.md)
- [Errors](reference/actors-errors.md)
- [Events](reference/actors-events.md)
- [External SQL Database](reference/actors-external-sql.md)
- [Fetch and WebSocket Handler](reference/actors-fetch-and-websocket-handler.md)
- [Helper Types](reference/actors-helper-types.md)
- [Input Parameters](reference/actors-input.md)
- [Lifecycle](reference/actors-lifecycle.md)
- [Low-Level HTTP Request Handler](reference/actors-request-handler.md)
- [Low-Level KV Storage](reference/actors-kv.md)
- [Low-Level WebSocket Handler](reference/actors-websocket-handler.md)
- [Metadata](reference/actors-metadata.md)
- [Next.js Quickstart](reference/actors-quickstart-next-js.md)
- [Node.js & Bun Quickstart](reference/actors-quickstart-backend.md)
- [React Quickstart](reference/actors-quickstart-react.md)
- [Scaling & Concurrency](reference/actors-scaling.md)
- [Sharing and Joining State](reference/actors-sharing-and-joining-state.md)
- [State](reference/actors-state.md)
- [Testing](reference/actors-testing.md)
- [Types](reference/actors-types.md)
- [Vanilla HTTP API](reference/actors-http-api.md)
- [Versions & Upgrades](reference/actors-versions.md)

### Clients

- [Next.js](reference/clients-next-js.md)
- [Node.js & Bun](reference/clients-javascript.md)
- [React](reference/clients-react.md)
- [Rust](reference/clients-rust.md)

### Connect

- [Deploy To Amazon Web Services Lambda](reference/connect-aws-lambda.md)
- [Deploying to AWS ECS](reference/connect-aws-ecs.md)
- [Deploying to Cloudflare Workers](reference/connect-cloudflare-workers.md)
- [Deploying to Freestyle](reference/connect-freestyle.md)
- [Deploying to Google Cloud Run](reference/connect-gcp-cloud-run.md)
- [Deploying to Hetzner](reference/connect-hetzner.md)
- [Deploying to Kubernetes](reference/connect-kubernetes.md)
- [Deploying to Railway](reference/connect-railway.md)
- [Deploying to Vercel](reference/connect-vercel.md)
- [Deploying to VMs & Bare Metal](reference/connect-vm-and-bare-metal.md)
- [Supabase](reference/connect-supabase.md)

### General

- [Actor Configuration](reference/general-actor-configuration.md)
- [Architecture](reference/general-architecture.md)
- [Cross-Origin Resource Sharing](reference/general-cors.md)
- [Documentation for LLMs & AI](reference/general-docs-for-llms.md)
- [Edge Networking](reference/general-edge.md)
- [Endpoints](reference/general-endpoints.md)
- [Environment Variables](reference/general-environment-variables.md)
- [HTTP Server](reference/general-http-server.md)
- [Logging](reference/general-logging.md)
- [MCP Server](reference/general-mcp.md)
- [Registry Configuration](reference/general-registry-configuration.md)
- [Runtime Modes](reference/general-runtime-modes.md)

### Self Hosting

- [Configuration](reference/self-hosting-configuration.md)
- [Docker Compose](reference/self-hosting-docker-compose.md)
- [Docker Container](reference/self-hosting-docker-container.md)
- [File System](reference/self-hosting-filesystem.md)
- [Installing Rivet Engine](reference/self-hosting-install.md)
- [Kubernetes](reference/self-hosting-kubernetes.md)
- [Multi-Region](reference/self-hosting-multi-region.md)
- [PostgreSQL](reference/self-hosting-postgres.md)
- [Railway Deployment](reference/self-hosting-railway.md)

