import { Elysia } from "elysia";
import { createClient } from "rivetkit/client";
import { registry } from "./actors.ts";

const client = createClient<typeof registry>();

const app = new Elysia()
	.all("/api/rivet/*", (c) => registry.handler(c.request))
	.get("/increment/:name", async ({ params }) => {
		const counter = client.counter.getOrCreate(params.name);
		const newCount = await counter.increment(1);
		return `New Count: ${newCount}`;
	});

export default app;
