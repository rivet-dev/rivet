# Next.js Quickstart

> Source: `src/content/docs/actors/quickstart/next-js.mdx`
> Canonical URL: https://rivet.gg/docs/actors/quickstart/next-js
> Description: Get started with Rivet Actors in Next.js

---
### Create a Next.js App

```sh
npx create-next-app@latest my-app
cd my-app
```

### Install RivetKit

### Create an Actor

Create a file at `src/rivet/registry.ts` with a simple counter actor:

```ts {{"title":"src/rivet/registry.ts"}}
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

### Setup Rivet API route

Create a file at `src/app/api/rivet/[...all]/route.ts` to setup the API routes:

```ts {{"title":"src/app/api/rivet/[...all]/route.ts"}} @nocheck
import { toNextHandler } from "@rivetkit/next-js";
import { registry } from "@/rivet/registry";

export const maxDuration = 300;

export const { GET, POST, PUT, PATCH, HEAD, OPTIONS } = toNextHandler(registry);
```

### Use the Actor in a component

Create a Counter component and add it to your page:

```tsx src/components/Counter.tsx @nocheck
"use client";

import { createRivetKit } from "@rivetkit/next-js/client";
import type { registry } from "@/rivet/registry";
import { useState } from "react";

export const { useActor } = createRivetKit<typeof registry>(
	process.env.NEXT_RIVET_ENDPOINT ?? "http://localhost:3000/api/rivet",
);

export function Counter() {
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

```tsx src/app/page.tsx @nocheck
import { Counter } from "@/components/Counter";

export default function Home() {
	return (
		<main>
			<h1>My App</h1>
			<Counter />
		</main>
	);
}
```

For information about the Next.js client API, see the [React Client API Reference](/docs/clients/react).

### Deploy to Vercel

See the [Vercel deployment guide](/docs/connect/vercel) for detailed instructions on deploying your Rivet app to Vercel.

_Source doc path: /docs/actors/quickstart/next-js_
