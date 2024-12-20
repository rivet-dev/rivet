# Rivet Actor Core

Related:

- [**Documentation**](https://rivet.gg/docs)
- [**API documentation**](https://rivet.gg/docs/api)
- [**@rivet-gg/actor**](https://jsr.io/@rivet-gg/actor)

## Usage

See the [setup guide](https://rivet.gg/docs/setup) for more information.

See the [API documentation](https://rivet.gg/docs/api) on how to create this actor.

## Example

```typescript
import { Hono } from "hono";
import { upgradeWebSocket } from "hono/deno";
import { assertExists } from "@std/assert";


// Start server
export default {
	async start(ctx) {
		// Start basic HTTP server
		const app = new Hono();

		app.get("/kv-test", (c) => {
			// Access the KV API
			await ctx.kv.put("foo", "bar");
			return c.text("done");
		});

		// Find port
		const portEnv = Deno.env.get("PORT_HTTP");
		assertExists(portEnv, "missing PORT_HTTP");
		const port = Number.parseInt(portEnv);

		// Start server
		console.log(`Listening on port ${port}`);
		const server = Deno.serve({ port }, app.fetch);
		await server.finished;
	},
};

```

