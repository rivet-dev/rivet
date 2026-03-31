import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./index.ts";

const client = createClient<typeof registry>();

const app = new Hono();

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

export default app;

// Start server when run directly
if (import.meta.url === `file://${process.argv[1]}`) {
	const { serve } = await import("@hono/node-server");
	const port = 3000;
	serve({ fetch: app.fetch, port }, () =>
		console.log(`Server running at http://localhost:${port}`),
	);
}
