import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

registry.startRunner();
const client = createClient<typeof registry>();

// Setup router
const app = new Hono();

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(String(newCount));
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
