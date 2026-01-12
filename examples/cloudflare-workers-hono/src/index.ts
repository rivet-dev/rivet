import { type Client, createHandler } from "@rivetkit/cloudflare-workers";
import { Hono } from "hono";
import { registry } from "./registry";

// Setup router
const app = new Hono<{ Bindings: { RIVET: Client<typeof registry> } }>();

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const client = c.env.RIVET;

	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

const { handler, ActorHandler } = createHandler(registry, { fetch: app.fetch });
export { handler as default, ActorHandler };
