import { Elysia } from "elysia";
import { createClient } from "rivetkit/client";
import { registry } from "./index.ts";

const client = createClient<typeof registry>();

const app = new Elysia()
	.get("/increment/:name", async ({ params }) => {
		const counter = client.counter.getOrCreate(params.name);
		const newCount = await counter.increment(1);
		return `New Count: ${newCount}`;
	});
const handler = registry.fetchHandler({
	path: "/api/rivet",
	dev: "http://127.0.0.1:3000/api/rivet",
});
app.all("/api/rivet/*", (c) => handler(c.request));

export default app;
