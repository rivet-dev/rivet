# Rivet Actor Core

Core low-level types for working with the Rivet Actor runtime.

**This package is for advanced low-level usage of Rivet. If getting started, please use the
[Actor SDK](https://jsr.io/@rivet-gg/actor).**

## Getting Started

- [Setup Guide](https://rivet.gg/docs/setup)
- [Documentation](https://rivet.gg/docs)
- [Examples](https://github.com/rivet-gg/rivet/tree/main/examples)
- [API documentation](https://rivet.gg/docs/api)

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

## Related Packages

- [Actor SDK (@rivet-gg/actor)](https://jsr.io/@rivet-gg/actor)

## Community & Support

- Join our [Discord](https://rivet.gg/discord)
- Follow us on [X](https://x.com/rivet_gg)
- Follow us on [Bluesky](https://bsky.app/profile/rivet-gg.bsky.social)
- File bug reports in [GitHub Issues](https://github.com/rivet-gg/rivet/issues)
- Post questions & ideas in [GitHub Discussions](https://github.com/orgs/rivet-gg/discussions)

## License

Apache 2.0
