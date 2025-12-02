import { createClient } from "rivetkit/client";
import type { registry } from "../src/rivet/registry";

async function main() {
	const client = createClient<typeof registry>(
		"http://localhost:3000/api/rivet",
	);

	const counter = client.counter.getOrCreate().connect();

	counter.on("newCount", (count: number) => console.log("Event:", count));

	while (true) {
		const out = await counter.increment(1);
		console.log("RPC:", out);

		await new Promise((resolve) => setTimeout(resolve, 1000));
	}
}

main();
