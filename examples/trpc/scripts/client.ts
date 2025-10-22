import { createTRPCClient, httpBatchLink } from "@trpc/client";
import type { AppRouter } from "../src/server.js";

// Create tRPC client
const client = createTRPCClient<AppRouter>({
	links: [
		httpBatchLink({
			url: "http://localhost:3001",
		}),
	],
});

async function main() {
	console.log("🚀 tRPC Client Demo");

	try {
		// Increment counter
		console.log("Incrementing counter 'demo'...");
		const result = await client.increment.mutate({ name: "demo" });
		console.log("New count:", result);

		// Increment again
		console.log("Incrementing counter 'demo' again...");
		const result2 = await client.increment.mutate({ name: "demo" });
		console.log("New count:", result2);

		console.log("✅ Demo completed!");
	} catch (error) {
		console.error("❌ Error:", error);
		process.exit(1);
	}
}

main().catch(console.error);
