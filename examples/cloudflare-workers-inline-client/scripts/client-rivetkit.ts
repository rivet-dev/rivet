import { createClient } from "rivetkit/client";
import type { registry } from "../src/registry";

// Create RivetKit client
const client = createClient<typeof registry>(
	process.env.RIVET_ENDPOINT ?? "http://localhost:8787/rivet",
);

async function main() {
	console.log("🚀 Cloudflare Workers Client Demo");

	try {
		const counter = client.counter.getOrCreate("demo").connect();

		for (let i = 0; i < 3; i++) {
			// Increment counter
			console.log("Incrementing counter...");
			const result1 = await counter.increment(1);
			console.log("New count:", result1);
		}

		await counter.dispose();

		console.log("✅ Demo completed!");
	} catch (error) {
		console.error("❌ Error:", error);
		process.exit(1);
	}
}

main().catch(console.error);
