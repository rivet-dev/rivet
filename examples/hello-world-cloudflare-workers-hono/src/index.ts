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

interface Env {
	RIVET_ENDPOINT: string;
}

const app = new Hono<{ Bindings: Env }>();

app.get("/", (c) => c.text("Hello from Hono + Rivet Actors!"));

app.post("/increment/:name", async (c) => {
	const client = createClient<typeof registry>({
		endpoint: c.env.RIVET_ENDPOINT,
	});
	const count = await client.counter
		.getOrCreate(c.req.param("name"))
		.increment(1);
	return c.json({ count });
});

// Mount your Hono app alongside Rivet. The Rivet manager API stays on
// `/api/rivet`; every other route is handled by `app`.
export default createHandler(registry, { fetch: app.fetch });
