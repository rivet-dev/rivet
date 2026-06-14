import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT ?? "http://localhost:6420",
});

async function main() {
	const counter = client.counter.getOrCreate("demo");
	console.log(`increment(3) -> ${await counter.increment(3)}`);
	console.log(`getCount() -> ${await counter.getCount()}`);
	console.log("round trip ok");
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
