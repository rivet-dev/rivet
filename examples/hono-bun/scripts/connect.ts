import { createClient } from "rivetkit/client";
import type { registry } from "../src/registry";

async function main() {
	const client = createClient<typeof registry>(
		"http://localhost:8080/api/rivet",
	);

	const counter = client.counter.getOrCreate().connect();

	counter.on("newCount", (count: number) => console.log("Event:", count));

	for (let i = 0; i < 5; i++) {
		const out = await counter.increment(5);
		console.log("RPC:", out);

		await new Promise((resolve) => setTimeout(resolve, 1000));
	}

	await new Promise((resolve) => setTimeout(resolve, 10000));
	await counter.dispose();
}

main();
