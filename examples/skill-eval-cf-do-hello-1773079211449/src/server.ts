import { Hono } from "hono";
import { registry } from "./actors.ts";

const app = new Hono();

app.get("/", async (c) => {
	const client = registry.client();
	const handle = client.myActor.getOrCreate([
		new URL(c.req.url).pathname,
	]);
	const greeting = await handle.sayHello();
	return c.text(greeting);
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
