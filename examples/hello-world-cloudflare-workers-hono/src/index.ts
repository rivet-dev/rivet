import { createHandler, setup } from "@rivetkit/cloudflare-workers";
import { Hono } from "hono";
import { actor } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

export const registry = setup({ use: { counter } });

const app = new Hono();

app.get("/", (c) => c.text("Hello from Hono + Rivet Actors!"));

app.post("/increment/:name", async (c) => {
	// `createClient` reads RIVET_ENDPOINT from the environment. `rivet dev`
	// passes it automatically; in production set it as a Worker var or secret. It
	// falls back to the local engine at http://localhost:6420 when unset.
	const client = createClient<typeof registry>();
	const count = await client.counter
		.getOrCreate(c.req.param("name"))
		.increment(1);
	return c.json({ count });
});

// Mount your Hono app alongside Rivet. The Rivet manager API stays on
// `/api/rivet`; every other route is handled by `app`.
export default createHandler(registry, { fetch: app.fetch });
