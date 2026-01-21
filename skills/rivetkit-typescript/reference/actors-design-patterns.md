# Design Patterns

> Source: `src/content/docs/actors/design-patterns.mdx`
> Canonical URL: https://rivet.gg/docs/actors/design-patterns
> Description: Common patterns and anti-patterns for building scalable actor systems.

---
## How Actors Scale

Actors are inherently scalable because of how they're designed:

- **Isolated state:** Each actor manages its own private data. No shared state means no conflicts and no locks, so actors run concurrently without coordination.
- **Actor-to-actor communication:** Actors interact through [actions](/docs/actors/actions) and [events](/docs/actors/events), so they don't need to coordinate access to shared data. This makes it easy to distribute them across machines.
- **Small, focused units:** Each actor handles a limited scope (a single user, document, or chat room), so load naturally spreads across many actors rather than concentrating in one place.
- **Horizontal scaling:** Adding more machines automatically distributes actors across them.

These properties form the foundation for the patterns described below.

## Actor Per Entity

The core pattern is creating one actor per entity in your system. Each actor represents a single user, document, chat room, or other distinct object. This keeps actors small, independent, and easy to scale.

**Good examples**

- `User`: Manages user profile, preferences, and authentication
- `Document`: Handles document content, metadata, and versioning
- `ChatRoom`: Manages participants and message history

**Bad examples**

- `Application`: Too broad, handles everything
- `DocumentWordCount`: Too granular, should be part of Document actor

## Coordinator & Data Actors

Actors scale by splitting state into isolated entities. However, it's common to need to track and coordinate actors in a central place. This is where coordinator actors come in.

**Data actors** handle the main logic in your application. Examples: chat rooms, user sessions, game lobbies.

**Coordinator actors** track other actors. Think of them as an index of data actors. Examples: a list of chat rooms, a list of active users, a list of game lobbies.

**Example: Chat Room Coordinator**

### Actor

```ts
import { actor, setup } from "rivetkit";

// Data actor: handles messages and connections
const chatRoom = actor({
  state: { messages: [] as { sender: string; text: string }[] },
  actions: {
    sendMessage: (c, sender: string, text: string) => {
      const message = { sender, text };
      c.state.messages.push(message);
      c.broadcast("newMessage", message);
      return message;
    },
    getHistory: (c) => c.state.messages,
  },
});

// Coordinator: indexes chat rooms
const chatRoomList = actor({
  state: { chatRoomIds: [] as string[] },
  actions: {
    createChatRoom: async (c, name: string) => {
      const client = c.client<typeof registry>();
      // Create the chat room actor and get its ID
      const handle = await client.chatRoom.create([name]);
      const actorId = await handle.resolve();
      // Track it in the list
      c.state.chatRoomIds.push(actorId);
      return actorId;
    },
    listChatRooms: (c) => c.state.chatRoomIds,
  },
});

const registry = setup({
  use: { chatRoom, chatRoomList },
});
```

### Client

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: { messages: [] as { sender: string; text: string }[] },
  actions: {
    sendMessage: (c, sender: string, text: string) => {
      const message = { sender, text };
      c.state.messages.push(message);
      return message;
    },
    getHistory: (c) => c.state.messages,
  },
});

const chatRoomList = actor({
  state: { chatRoomIds: [] as string[] },
  actions: {
    createChatRoom: async (c, name: string) => "room-id",
    listChatRooms: (c) => c.state.chatRoomIds,
  },
});

const registry = setup({ use: { chatRoom, chatRoomList } });
const client = createClient<typeof registry>("http://localhost:8080");

// Create a new chat room via coordinator
const coordinator = client.chatRoomList.getOrCreate(["main"]);
const actorId = await coordinator.createChatRoom("general");

// Get list of all chat rooms
const chatRoomIds = await coordinator.listChatRooms();

// Connect to a chat room using its ID
const chatRoomHandle = client.chatRoom.getForId(actorId);
await chatRoomHandle.sendMessage("alice", "Hello!");
const history = await chatRoomHandle.getHistory();
```

## Sharding

Sharding splits a single actor's workload across multiple actors based on a key. Use this when one actor can't handle all the load or data for an entity.

**How it works:**
- Partition data using a shard key (user ID, region, time bucket, or random)
- Requests are routed to shards based on the key
- Shards operate independently without coordination

**Example: Sharding by Time**

### Actor

```ts
import { actor, setup } from "rivetkit";

interface Event {
  type: string;
  url: string;
}

const hourlyAnalytics = actor({
  state: { events: [] as Event[] },
  actions: {
    trackEvent: (c, event: Event) => {
      c.state.events.push(event);
    },
    getEvents: (c) => c.state.events,
  },
});

export const registry = setup({
  use: { hourlyAnalytics },
});
```

### Client

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface Event {
  type: string;
  url: string;
}

const hourlyAnalytics = actor({
  state: { events: [] as Event[] },
  actions: {
    trackEvent: (c, event: Event) => {
      c.state.events.push(event);
    },
  },
});

const registry = setup({ use: { hourlyAnalytics } });
const client = createClient<typeof registry>("http://localhost:8080");

// Shard by hour: hourlyAnalytics:2024-01-15T00, hourlyAnalytics:2024-01-15T01
const shardKey = new Date().toISOString().slice(0, 13); // "2024-01-15T00"
const analytics = client.hourlyAnalytics.getOrCreate([shardKey]);
await analytics.trackEvent({ type: "page_view", url: "/home" });
```

**Example: Random Sharding**

### Actor

```ts
import { actor, setup } from "rivetkit";

const rateLimiter = actor({
  state: { requests: {} as Record<string, number> },
  actions: {
    checkLimit: (c, userId: string, limit: number) => {
      const count = c.state.requests[userId] ?? 0;
      if (count >= limit) return false;
      c.state.requests[userId] = count + 1;
      return true;
    },
  },
});

export const registry = setup({
  use: { rateLimiter },
});
```

### Client

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const rateLimiter = actor({
  state: { requests: {} as Record<string, number> },
  actions: {
    checkLimit: (c, userId: string, limit: number) => {
      const count = c.state.requests[userId] ?? 0;
      if (count >= limit) return false;
      c.state.requests[userId] = count + 1;
      return true;
    },
  },
});

const registry = setup({ use: { rateLimiter } });
const client = createClient<typeof registry>("http://localhost:8080");

// Shard randomly: rateLimiter:shard-0, rateLimiter:shard-1, rateLimiter:shard-2
const shardKey = `shard-${Math.floor(Math.random() * 3)}`;
const limiter = client.rateLimiter.getOrCreate([shardKey]);
const allowed = await limiter.checkLimit("user-123", 100);
```

Choose shard keys that distribute load evenly. Note that cross-shard queries require coordination.

## Fan-In & Fan-Out

Fan-in and fan-out are patterns for distributing work and aggregating results.

**Fan-Out**: One actor spawns work across multiple actors. Use for parallel processing or broadcasting updates.

**Fan-In**: Multiple actors send results to one aggregator. Use for collecting results or reducing data.

**Example: Map-Reduce**

### Actor

```ts
import { actor, setup } from "rivetkit";

interface Task {
  id: string;
  data: string;
}

interface Result {
  taskId: string;
  output: string;
}

// Coordinator fans out tasks, then fans in results
const coordinator = actor({
  state: { results: [] as Result[] },
  actions: {
    // Fan-out: distribute work in parallel
    startJob: async (c, tasks: Task[]) => {
      const client = c.client<typeof registry>();
      await Promise.all(
        tasks.map(task => client.worker.getOrCreate(task.id).process(task))
      );
    },
    // Fan-in: collect results
    reportResult: (c, result: Result) => {
      c.state.results.push(result);
    },
    getResults: (c) => c.state.results,
  },
});

const worker = actor({
  state: {},
  actions: {
    process: async (c, task: Task) => {
      const result = { taskId: task.id, output: `Processed ${task.data}` };
      const client = c.client<typeof registry>();
      await client.coordinator.getOrCreate("main").reportResult(result);
    },
  },
});

export const registry = setup({
  use: { coordinator, worker },
});
```

### Client

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface Task {
  id: string;
  data: string;
}

interface Result {
  taskId: string;
  output: string;
}

const coordinator = actor({
  state: { results: [] as Result[] },
  actions: {
    startJob: async (c, tasks: Task[]) => {},
    reportResult: (c, result: Result) => { c.state.results.push(result); },
    getResults: (c) => c.state.results,
  },
});

const worker = actor({
  state: {},
  actions: {
    process: async (c, task: Task) => {},
  },
});

const registry = setup({ use: { coordinator, worker } });
const client = createClient<typeof registry>("http://localhost:8080");

const coordinatorHandle = client.coordinator.getOrCreate(["main"]);

// Start a job with multiple tasks
await coordinatorHandle.startJob([
  { id: "task-1", data: "..." },
  { id: "task-2", data: "..." },
  { id: "task-3", data: "..." },
]);

// Results are collected as workers report back
const results = await coordinatorHandle.getResults();
```

## Integrating With External Databases & APIs

Actors can integrate with external resources like databases or external APIs.

### Loading State

Load external data during actor initialization using `createVars`. This keeps your actor's persisted state clean while caching expensive lookups.

Use this when:

- Fetching user profiles, configs, or permissions from a database
- Loading data that changes externally and shouldn't be persisted
- Caching expensive API calls or computations

**Example: Loading User Profile**

### Actor

```ts
import { actor, setup } from "rivetkit";

interface User {
  id: string;
  email: string;
  name: string;
}

// Mock database interface for demonstration
const db = {
  users: {
    findById: async (id: string): Promise<User> => ({ id, email: "user@example.com", name: "User" }),
    update: async (id: string, data: Partial<User>) => {},
  },
};

const userSession = actor({
  state: { requestCount: 0 },

  // createVars runs on every wake (after restarts, crashes, or sleep), so
  // external data stays fresh.
  createVars: async (c): Promise<{ user: User }> => {
    // Load from database on every wake
    const user = await db.users.findById(c.key.join("-"));
    return { user };
  },

  actions: {
    getProfile: (c) => {
      c.state.requestCount++;
      return c.vars.user;
    },
    updateEmail: async (c, email: string) => {
      c.state.requestCount++;
      await db.users.update(c.key.join("-"), { email });
      // Refresh cached data
      c.vars.user = await db.users.findById(c.key.join("-"));
    },
  },
});

const registry = setup({
  use: { userSession },
});
```

### Client

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface User {
  id: string;
  email: string;
  name: string;
}

const userSession = actor({
  state: { requestCount: 0 },
  createVars: () => ({ user: null as User | null }),
  actions: {
    getProfile: (c) => c.vars.user,
    updateEmail: async (c, email: string) => {},
  },
});

const registry = setup({ use: { userSession } });
const client = createClient<typeof registry>("http://localhost:8080");

const session = client.userSession.getOrCreate(["user-123"]);

// Get profile (loaded from database on actor wake)
const profile = await session.getProfile();

// Update email (writes to database and refreshes cache)
await session.updateEmail("alice@example.com");
```

### Syncing State Changes

Use `onStateChange` to automatically sync actor state changes to external resources. This hook is called whenever the actor's state is modified.

Use this when:

- You need to mirror actor state in an external database
- Triggering external side effects when state changes
- Keeping external systems in sync with actor state

**Example: Syncing to Database**

### Actor

```ts
import { actor, setup } from "rivetkit";

// Mock database interface for demonstration
const db = {
  users: {
    insert: async (data: { id: string; email: string; createdAt: number }) => {},
    update: async (id: string, data: { email: string; lastActive: number }) => {},
  },
};

const userActor = actor({
  state: {
    email: "",
    lastActive: 0,
  },

  onCreate: async (c, input: { email: string }) => {
    // Insert into database on actor creation
    await db.users.insert({
      id: c.key.join("-"),
      email: input.email,
      createdAt: Date.now(),
    });
  },

  onStateChange: async (c, newState) => {
    // Sync any state changes to database
    await db.users.update(c.key.join("-"), {
      email: newState.email,
      lastActive: newState.lastActive,
    });
  },

  actions: {
    updateEmail: (c, email: string) => {
      c.state.email = email;
      c.state.lastActive = Date.now();
    },
    getUser: (c) => ({
      email: c.state.email,
      lastActive: c.state.lastActive,
    }),
  },
});

const registry = setup({
  use: { userActor },
});
```

### Client

```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const userActor = actor({
  state: { email: "", lastActive: 0 },
  actions: {
    updateEmail: (c, email: string) => {
      c.state.email = email;
      c.state.lastActive = Date.now();
    },
    getUser: (c) => ({
      email: c.state.email,
      lastActive: c.state.lastActive,
    }),
  },
});

const registry = setup({ use: { userActor } });
const client = createClient<typeof registry>("http://localhost:8080");

const user = await client.userActor.create(["user-123"], {
  input: { email: "alice@example.com" },
});

// Updates state and triggers onStateChange
await user.updateEmail("alice2@example.com");

const userData = await user.getUser();
```

`onStateChange` is called after every state modification, ensuring external resources stay in sync.

## Anti-Patterns

### "God" Actor

Avoid creating a single actor that handles everything. This defeats the purpose of the actor model and creates a bottleneck.

**Problem:**
```ts
import { actor } from "rivetkit";

// Bad: one actor doing everything
const app = actor({
  state: { users: {}, orders: {}, inventory: {}, analytics: {} },
  actions: {
    createUser: (c, user) => { /* ... */ },
    processOrder: (c, order) => { /* ... */ },
    updateInventory: (c, item) => { /* ... */ },
    trackEvent: (c, event) => { /* ... */ },
  },
});
```

**Solution:** Split into focused actors per entity (User, Order, Inventory, Analytics).

### Actor-Per-Request

Actors are designed to maintain state across multiple requests. Creating a new actor for each request wastes resources and loses the benefits of persistent state.

**Problem:**
```ts
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { Hono } from "hono";

const processor = actor({
  state: {},
  actions: {
    process: (c, body: unknown) => ({ processed: true }),
    destroy: (c) => {},
  },
});

const registry = setup({ use: { processor } });
const client = createClient<typeof registry>("http://localhost:8080");
const app = new Hono();

// Bad: creating an actor for each API request
app.post("/process", async (c) => {
  const actorHandle = client.processor.getOrCreate([crypto.randomUUID()]);
  const result = await actorHandle.process(await c.req.json());
  await actorHandle.destroy();
  return c.json(result);
});
```

**Solution:** Use actors for entities that persist (users, sessions, documents), not for one-off operations. For stateless request handling, use regular functions.

## API Reference

- [`ActorDefinition`](/typedoc/interfaces/rivetkit.mod.ActorDefinition.html) - Interface for pattern examples
- [`ActorContext`](/typedoc/interfaces/rivetkit.mod.ActorContext.html) - Context usage patterns
- [`ActionContext`](/typedoc/interfaces/rivetkit.mod.ActionContext.html) - Action patterns

_Source doc path: /docs/actors/design-patterns_
