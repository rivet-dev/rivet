# @rivetkit/cloudflare-workers

Cloudflare Workers integration for [RivetKit](https://rivet.dev) actors.

Host Rivet Actors on Cloudflare Workers with a single import. The wasm runtime
and the fetch-based WebSocket shim are wired automatically.

```ts
import { actor } from "rivetkit";
import { createHandler } from "@rivetkit/cloudflare-workers";

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount = 1) => (c.state.count += amount),
		getCount: (c) => c.state.count,
	},
});

export default createHandler({ use: { counter } });
```

Set `RIVET_ENDPOINT` in `wrangler.toml` `[vars]` (namespace and token may be
embedded in the URL as `https://namespace:token@host`).

## Mounting your own routes

Pass `fetch` to handle everything outside the Rivet manager API path
(`/api/rivet`). Use `setup` to get a typed registry so a `createClient` call is
fully typed:

```ts
import { createHandler, setup } from "@rivetkit/cloudflare-workers";
import { createClient } from "rivetkit/client";
import { Hono } from "hono";

const registry = setup({ use: { counter } });

const app = new Hono<{ Bindings: { RIVET_ENDPOINT: string } }>();
app.post("/increment/:name", async (c) => {
	const client = createClient<typeof registry>({ endpoint: c.env.RIVET_ENDPOINT });
	const count = await client.counter.getOrCreate(c.req.param("name")).increment(1);
	return c.json({ count });
});

export default createHandler(registry, { fetch: app.fetch });
```

Learn more at https://rivet.dev/docs.
