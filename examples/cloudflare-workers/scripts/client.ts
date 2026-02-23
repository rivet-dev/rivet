import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors";

// Create RivetKit client
const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT ?? "http://localhost:8787/api/rivet",
	disableMetadataLookup: true,
});

async function main() {
	console.log("🚀 Cloudflare Workers SQLite E2E Demo");

	try {
		const counterKey = "sqlite-demo";
		const counter = client.sqliteCounter.getOrCreate(counterKey);

		const initialCount = await counter.getCount();
		console.log("Initial count:", initialCount);

		const afterOne = await counter.increment(1);
		console.log("After +1:", afterOne);

		const afterFive = await counter.increment(5);
		console.log("After +5:", afterFive);

		const expected = initialCount + 6;
		if (afterFive !== expected) {
			throw new Error(
				`Unexpected count after increments. Expected ${expected}, got ${afterFive}.`,
			);
		}

		// Ensure the value persisted by re-resolving the actor handle.
		const counterAgain = client.sqliteCounter.getOrCreate(counterKey);
		const persistedCount = await counterAgain.getCount();
		console.log("Persisted count:", persistedCount);

		if (persistedCount !== expected) {
			throw new Error(
				`Persistence check failed. Expected ${expected}, got ${persistedCount}.`,
			);
		}

		console.log("✅ SQLite E2E check passed");
	} catch (error) {
		console.error("❌ Error:", error);
		process.exit(1);
	}
}

main().catch(console.error);
