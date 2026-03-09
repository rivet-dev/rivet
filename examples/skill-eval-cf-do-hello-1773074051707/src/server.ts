import { Hono } from "hono";
import { registry } from "./actors.ts";
import { createClient } from "rivetkit/client";

const app = new Hono();

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

// Replicate the original CF Worker behavior: GET any path calls sayHello
// on an actor keyed by the request pathname.
app.get("*", async (c) => {
	const pathname = new URL(c.req.url).pathname;
	const client = createClient<typeof registry>(registry);
	const handle = client.myActor.getOrCreate([pathname]);
	const greeting = await handle.sayHello();
	return c.text(greeting);
});

export default app;
