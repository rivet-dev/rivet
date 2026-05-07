import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./index.ts";

const client = createClient<typeof registry>();

const app = new Hono();
const handler = registry.fetchHandler({
	path: "/api/rivet",
	dev: "http://127.0.0.1:3000/api/rivet",
});

app.all("/api/rivet/*", (c) => handler(c.req.raw));

app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

export default app;
