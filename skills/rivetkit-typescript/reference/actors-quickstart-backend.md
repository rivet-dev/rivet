# Node.js & Bun Quickstart

> Source: `src/content/docs/actors/quickstart/backend.mdx`
> Canonical URL: https://rivet.gg/docs/actors/quickstart/backend
> Description: Get started with Rivet Actors in Node.js and Bun

---
## Steps

### Install Rivet

```sh
npm install rivetkit
```

### Create an Actor

Create a simple counter actor:

```ts {{"title":"registry.ts"}}
import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});
```

### Setup Server

### Run Server

### Step

### Connect To The Rivet Actor

This code can run either in your frontend or within your backend:

### TypeScript

```ts {{"title":"registry.ts"}} @hide
import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});
```

```ts {{"title":"client.ts"}}
import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

const client = createClient<typeof registry>();

// Get or create a counter actor for the key "my-counter"
const counter = client.counter.getOrCreate(["my-counter"]);

// Call actions
const count = await counter.increment(3);
console.log("New count:", count);

// Listen to realtime events
const connection = counter.connect();
connection.on("newCount", (newCount: number) => {
	console.log("Count changed:", newCount);
});

// Increment through connection
await connection.increment(1);
```

See the [JavaScript client documentation](/docs/clients/javascript) for more information.

### React

```ts {{"title":"registry.ts"}} @hide
import { actor, setup } from "rivetkit";

export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
	},
});

export const registry = setup({
	use: { counter },
});
```

```tsx {{"title":"Counter.tsx"}}
import { createRivetKit } from "@rivetkit/react";
import { useState } from "react";
import type { registry } from "./registry";

const { useActor } = createRivetKit<typeof registry>();

function Counter() {
	const [count, setCount] = useState(0);

	// Get or create a counter actor for the key "my-counter"
	const counter = useActor({
		name: "counter",
		key: ["my-counter"]
	});

	// Listen to realtime events
	counter.useEvent("newCount", (x: number) => setCount(x));

	const increment = async () => {
		// Call actions
		await counter.connection?.increment(1);
	};

	return (
		<div>
			<p>Count: {count}</p>
			<button onClick={increment}>Increment</button>
		</div>
	);
}
```

See the [React documentation](/docs/clients/react) for more information.

### Deploy

## Configuration Options

_Source doc path: /docs/actors/quickstart/backend_
