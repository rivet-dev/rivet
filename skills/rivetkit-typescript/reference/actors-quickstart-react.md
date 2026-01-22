# React Quickstart

> Source: `src/content/docs/actors/quickstart/react.mdx`
> Canonical URL: https://rivet.gg/docs/actors/quickstart/react
> Description: Build realtime React applications with Rivet Actors

---
## Steps

### Install Dependencies

```sh
npm install rivetkit @rivetkit/react
```

### Create Backend Actor

Create your actor registry on the backend:

```ts {{"title":"backend/registry.ts"}}
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

### Setup Backend Server

### Create React Frontend

Set up your React application:

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

For detailed information about the React client API, see the [React Client API Reference](/docs/clients/react).

### Setup Vite Configuration

Configure Vite for development:

```ts {{"title":"vite.config.ts"}} @nocheck
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
  },
})
```

### Run Your Application

Start both the backend and frontend:

**Terminal 1**: Start the backend

### Step

**Terminal 2**: Start the frontend

```sh Frontend
npx vite
```

Open `http://localhost:5173` in your browser. Try opening multiple tabs to see realtime sync in action.

### Deploy

## Configuration Options

_Source doc path: /docs/actors/quickstart/react_
