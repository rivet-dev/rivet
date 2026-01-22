# Clients

> Source: `src/content/docs/actors/clients.mdx`
> Canonical URL: https://rivet.gg/docs/actors/clients
> Description: Clients are used to get and communicate with actors from your application. Clients can be created from either your frontend or backend.

---
## Creating a Client

	
### Frontend Client

		For frontend applications or external services:

		

		```typescript {{"title":"JavaScript"}}
		import { createClient } from "rivetkit/client";
		import type { registry } from "./registry";  // Must use `type`

		const client = createClient<typeof registry>();
		```

		```tsx React @nocheck
		import { createRivetKit } from "@rivetkit/react";
		import type { registry } from "./registry";  // Must use `type`

		const { useActor } = createRivetKit<typeof registry>();
		```
		

		
			Clients include the following options:

			```typescript
			const client = createClient<typeof registry>({
				endpoint: "https://api.rivet.dev",
				namespace: "default",
				token: "my-token",
				encoding: "json",  // "json", "cbor", or "bare"
				headers: { "X-Custom-Header": "value" }
			});
			```
		
	

	
### Backend Client

		From your backend server that hosts the registry:

		```typescript
		import { createClient } from "rivetkit/client";
		import type { registry } from "./registry";

		const client = createClient<typeof registry>();
		```
	

	
### Actor-to-Actor

		From within an actor to communicate with other actors:

		```typescript
		import { actor, setup } from "rivetkit";

		const counter = actor({
			state: { count: 0 },
			actions: {
				increment: (c) => ++c.state.count
			}
		});

		const myActor = actor({
			state: {},
			actions: {
				callOtherActor: async (c) => {
					const client = c.client<typeof registry>();
					const counterHandle = client.counter.getOrCreate(["shared"]);
					return await counterHandle.increment();
				}
			}
		});

		const registry = setup({ use: { counter, myActor } });
		```

		Read more about [communicating between actors](/docs/actors/communicating-between-actors).
	

## Getting an Actor

### `getOrCreate`

Returns a handle to an existing actor or creates one if it doesn't exist:

```typescript {{"title":"JavaScript"}}
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

const counterHandle = client.counter.getOrCreate(["my-counter"]);
const count = await counterHandle.increment(5);
```

```tsx React @nocheck
const counter = useActor({
  name: "counter",
  key: ["my-counter"],
});

// Call actions through the connection
await counter.connection?.increment(5);
```

Pass initialization data when creating:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const game = actor({
  state: { gameMode: "", maxPlayers: 0 },
  actions: {}
});

const registry = setup({ use: { game } });
const client = createClient<typeof registry>();

const gameHandle = client.game.getOrCreate(["game-123"], {
  createWithInput: { gameMode: "tournament", maxPlayers: 8 }
});
```

```tsx React @nocheck
const game = useActor({
  name: "game",
  key: ["game-123"],
  createWithInput: { gameMode: "tournament", maxPlayers: 8 },
});
```

### `get`

Returns a handle to an existing actor or `null` if it doesn't exist:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const myActor = actor({
  state: {},
  actions: {
    someAction: (c) => "result"
  }
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>();

const handle = client.myActor.get(["actor-id"]);

if (handle) {
  await handle.someAction();
}
```

```tsx React @nocheck
import { createClient } from "rivetkit/client";
import { useMemo } from "react";
import type { registry } from "./registry";

// `get` is not currently supported with useActor.
// Use createClient with useMemo instead:

function MyComponent() {
  const client = useMemo(() => createClient<typeof registry>(), []);
  const handle = useMemo(() => client.myActor.get(["actor-id"]), [client]);

  // Use handle to call actions
}
```

### `create`

Creates a new actor, failing if one already exists with that key:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const game = actor({
  state: { gameMode: "" },
  actions: {}
});

const registry = setup({ use: { game } });
const client = createClient<typeof registry>();

const newGame = await client.game.create(["game-456"], {
  input: { gameMode: "classic" }
});
```

```tsx React @nocheck
import { createClient } from "rivetkit/client";
import { useMemo, useEffect, useState } from "react";
import type { registry } from "./registry";

// `create` is not currently supported with useActor.
// Use createClient with useMemo instead:

function MyComponent() {
  const client = useMemo(() => createClient<typeof registry>(), []);
  const [handle, setHandle] = useState(null);

  useEffect(() => {
    client.game.create(["game-456"], {
      input: { gameMode: "classic" }
    }).then(setHandle);
  }, [client]);

  // Use handle to call actions
}
```

### `getForId`

Connect to an actor using its internal ID:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const myActor = actor({
  state: {},
  actions: {}
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>();

const actorHandle = client.myActor.getForId("lrysjam017rhxofttna2x5nzjml610");
```

```tsx React @nocheck
import { createClient } from "rivetkit/client";
import { useMemo } from "react";
import type { registry } from "./registry";

// `getForId` is not currently supported with useActor.
// Use createClient with useMemo instead:

function MyComponent({ actorId }: { actorId: string }) {
  const client = useMemo(() => createClient<typeof registry>(), []);
  const handle = useMemo(() => client.myActor.getForId(actorId), [client, actorId]);

  // Use handle to call actions
}
```

Prefer using keys over internal IDs for simplicity.

## Calling Actions

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      return c.state.count;
    },
    getCount: (c) => c.state.count,
    reset: (c) => { c.state.count = 0; }
  }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>();

const counterHandle = client.counter.getOrCreate(["my-counter"]);

const count = await counterHandle.increment(5);
const value = await counterHandle.getCount();
await counterHandle.reset();
```

```tsx React @nocheck
const counter = useActor({
  name: "counter",
  key: ["my-counter"],
});

// Call actions through the connection
const count = await counter.connection?.increment(5);
const value = await counter.connection?.getCount();
await counter.connection?.reset();
```

In JavaScript, actions called without `connect()` are stateless. Each call is independent without a persistent connection. In React, `useActor` automatically manages a persistent connection.

## Connecting to an Actor

For real-time use cases, establish a persistent connection to the actor:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.count += amount;
      c.broadcast("countChanged", c.state.count);
      return c.state.count;
    }
  }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>();

const counterHandle = client.counter.getOrCreate(["live-counter"]);
const conn = counterHandle.connect();

// Listen for events
conn.on("countChanged", (newCount: number) => {
  console.log("Count updated:", newCount);
});

// Call actions through the connection
await conn.increment(1);
```

```tsx React @nocheck
const [count, setCount] = useState(0);

const counter = useActor({
  name: "counter",
  key: ["live-counter"],
});

// Listen for events
counter.useEvent("countChanged", (newCount: number) => {
  setCount(newCount);
});

// Call actions through the connection
await counter.connection?.increment(1);
```

## Subscribing to Events

Listen for events from connected actors:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface Message {
  from: string;
  text: string;
}

const chatRoom = actor({
  state: { messages: [] as Message[] },
  actions: {
    sendMessage: (c, from: string, text: string) => {
      c.state.messages.push({ from, text });
      c.broadcast("messageReceived", { from, text });
    },
    startGame: (c) => {
      c.broadcast("gameStarted");
    }
  }
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>();

const conn = client.chatRoom.getOrCreate(["general"]).connect();

// Listen for events
conn.on("messageReceived", (message: Message) => {
  console.log(`${message.from}: ${message.text}`);
});

// Listen once
conn.once("gameStarted", () => {
  console.log("Game has started!");
});
```

```tsx React @nocheck
const [messages, setMessages] = useState<Message[]>([]);

const chatRoom = useActor({
  name: "chatRoom",
  key: ["general"],
});

// Listen for events (automatically cleaned up on unmount)
chatRoom.useEvent("messageReceived", (message) => {
  setMessages((prev) => [...prev, message]);
});
```

## Full-Stack Type Safety

Import types from your registry for end-to-end type safety:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

// Define actors in a shared registry file
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

// In your client code, import the registry type
const client = createClient<typeof registry>();

// IDE autocomplete shows available actors and actions
const counterHandle = client.counter.getOrCreate(["my-counter"]);
const count = await counterHandle.increment(5);
```

```tsx React @nocheck
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./registry";

const { useActor } = createRivetKit<typeof registry>();

// IDE autocomplete shows available actors and actions
const counter = useActor({ name: "counter", key: ["my-counter"] });
const count = await counter.connection?.increment(5);
```

Use `import type` to avoid accidentally bundling backend code in your frontend.

## Advanced

### Disposing Clients & Connections

Dispose clients to close all connections:

```typescript
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const myActor = actor({
  state: {},
  actions: {}
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>();

// ... use client ...

await client.dispose();
```

Dispose individual connections when finished:

```typescript
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const myActor = actor({
  state: {},
  actions: {
    action: (c) => "result"
  }
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>();

const actorHandle = client.myActor.getOrCreate(["example"]);
const conn = actorHandle.connect();

const handler = (data: unknown) => console.log(data);

try {
  conn.on("event", handler);
  await conn.action();
} finally {
  await conn.dispose();
}
```

When using `useActor` in React, connections are automatically disposed when the component unmounts. No manual cleanup is required.

### Connection Parameters

Pass custom data to the actor when connecting:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: {},
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>();

const chat = client.chatRoom.getOrCreate(["general"], {
  params: {
    userId: "user-123",
    displayName: "Alice"
  }
});
```

```tsx React @nocheck
const chat = useActor({
  name: "chatRoom",
  key: ["general"],
  params: {
    userId: "user-123",
    displayName: "Alice"
  },
});
```

### Authentication

Pass authentication tokens when connecting:

```typescript {{"title":"JavaScript"}}
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: {},
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>();

const chat = client.chatRoom.getOrCreate(["general"], {
  params: {
    authToken: "jwt-token-here"
  }
});
```

```tsx React @nocheck
const chat = useActor({
  name: "chatRoom",
  key: ["general"],
  params: {
    authToken: "jwt-token-here"
  },
});
```

See [authentication](/docs/actors/authentication) for more details.

### Error Handling

```typescript {{"title":"JavaScript (Connection)"}}
import { actor, setup } from "rivetkit";
import { ActorError, createClient } from "rivetkit/client";

const myActor = actor({
  state: {},
  actions: {
    protectedAction: (c) => "result"
  }
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>();

const actorHandle = client.myActor.getOrCreate(["example"]);
const conn = actorHandle.connect();

conn.on("error", (error: ActorError) => {
  if (error.code === "forbidden") {
    console.log("Redirecting to login");
  }
});
```

```typescript {{"title":"JavaScript (Stateless)"}}
import { actor, setup } from "rivetkit";
import { ActorError, createClient } from "rivetkit/client";

const myActor = actor({
  state: {},
  actions: {
    protectedAction: (c) => "result"
  }
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>();

const actorHandle = client.myActor.getOrCreate(["example"]);

try {
  const result = await actorHandle.protectedAction();
} catch (error) {
  if (error instanceof ActorError && error.code === "forbidden") {
    console.log("Redirecting to login");
  }
}
```

```tsx React @nocheck
import { ActorError } from "rivetkit/client";

const actor = useActor({ name: "myActor", key: ["id"] });

const handleAction = async () => {
  try {
    await actor.connection?.protectedAction();
  } catch (error) {
    if (error instanceof ActorError && error.code === "forbidden") {
      window.location.href = "/login";
    }
  }
};
```

See [errors](/docs/actors/errors) for more details.

### Actor Resolution

`get` and `getOrCreate` return immediately without making a network request. The actor is resolved lazily when you call an action or `connect()`.

To explicitly resolve an actor and get its ID, use `resolve()`:

```typescript
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: {}
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>();

const handle = client.counter.getOrCreate(["my-counter"]);
const actorId = await handle.resolve();
console.log(actorId); // "lrysjam017rhxofttna2x5nzjml610"
```

## API Reference

- [`createClient`](/typedoc/functions/rivetkit.client_mod.createClient.html) - Function to create clients
- [`Client`](/typedoc/types/rivetkit.mod.Client.html) - Client type
- [`ActorHandle`](/typedoc/types/rivetkit.client_mod.ActorHandle.html) - Handle for interacting with actors
- [`ActorConn`](/typedoc/types/rivetkit.client_mod.ActorConn.html) - Connection to actors
- [`ClientRaw`](/typedoc/interfaces/rivetkit.client_mod.ClientRaw.html) - Raw client interface

_Source doc path: /docs/actors/clients_
