import { createClient } from "rivetkit/client";
import type { registry } from "../src/rivet/actors";

async function main() {
	const endpoint = process.env.RIVET_ENDPOINT ?? "http://localhost:3000/api/rivet";
	const client = createClient<typeof registry>({
		endpoint,
	});
	console.log("Using endpoint:", endpoint);

	const counterKey = ["sqlite-e2e"];
	const counter = client.sqliteCounter.getOrCreate(counterKey);

	const initialCount = await counter.getCount();
	console.log("Initial count:", initialCount);

	const afterTwo = await counter.increment(2);
	console.log("After +2:", afterTwo);

	const afterFive = await counter.increment(3);
	console.log("After +3:", afterFive);

	const expected = initialCount + 5;
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
}

main().catch((error) => {
	console.error("❌ Error:", error);
	process.exit(1);
});
