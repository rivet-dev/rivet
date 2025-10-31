import { Elysia } from "elysia";
import { registry } from "./registry";

const { client } = registry.start();

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

console.log("Listening at http://localhost:6420");
