import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { registry } from "./registry";

const { client } = registry.start();

// Setup router
const app = new Hono();

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

serve({ fetch: app.fetch, port: 8080 });
console.log("Listening on port 8080");
