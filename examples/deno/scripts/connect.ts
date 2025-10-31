import { createClient } from "rivetkit/client";
import type { Registry } from "../src/registry.ts";

async function main() {
	const client = createClient<Registry>("http://localhost:6420");

	const counter = client.counter.getOrCreate().connect();

	for (let i = 0; i < 5; i++) {
		const out = await counter.increment(5);
		console.log("RPC:", out);

		await new Promise((resolve) => setTimeout(resolve, 1000));
	}
}

main();
