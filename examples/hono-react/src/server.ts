import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { serveStatic } from "@hono/node-server/serve-static";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

registry.startRunner();
const client = createClient<typeof registry>();

// Setup router
const app = new Hono();

// Serve static files from dist (built frontend)
app.use("/*", serveStatic({ root: "./dist" }));

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

serve({ fetch: app.fetch, port: 8080 });
console.log("Listening on port 8080");
