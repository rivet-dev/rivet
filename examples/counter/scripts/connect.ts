import { createClient } from "rivetkit/client";
import type { Registry } from "../src/registry";

async function main() {
	const client = createClient<Registry>("http://localhost:6420");

	const counter = client.counter.getOrCreate().connect();

	counter.on("newCount", (count: number) => console.log("Event:", count));

	while (true) {
		const out = await counter.increment(1);
		console.log("RPC:", out);

		await new Promise((resolve) => setTimeout(resolve, 1000));
	}

	await new Promise((resolve) => setTimeout(resolve, 10000));
	await counter.dispose();
}

main();
