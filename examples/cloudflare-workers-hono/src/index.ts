import { createHandler } from "@rivetkit/cloudflare-workers";
import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

const client = createClient<typeof registry>();

// Setup router
const app = new Hono();

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

const { handler, ActorHandler } = createHandler(registry, { fetch: app.fetch });
export { handler as default, ActorHandler };
