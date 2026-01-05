import { Elysia } from "elysia";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

registry.startRunner();
const client = createClient<typeof registry>();

// Setup router
new Elysia()
	// Example HTTP endpoint
	.post("/increment/:name", async ({ params }) => {
		const name = params.name;

		const counter = client.counter.getOrCreate(name);
		const newCount = await counter.increment(1);

		return `New Count: ${newCount}`;
	})
	.listen(8080);

console.log("Listening at http://localhost:8080");
