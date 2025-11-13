const baseUrl = process.env.BASE_URL ?? "http://localhost:8787";

async function main() {
	console.log("🚀 Cloudflare Workers Client Demo");

	try {
		// Increment counter 'demo'
		console.log("Incrementing counter 'demo'...");
		const response1 = await fetch(`${baseUrl}/increment/demo`, {
			method: "POST",
		});
		const result1 = await response1.text();
		console.log(result1);

		// Increment counter 'demo' again
		console.log("Incrementing counter 'demo' again...");
		const response2 = await fetch(`${baseUrl}/increment/demo`, {
			method: "POST",
		});
		const result2 = await response2.text();
		console.log(result2);

		// Increment counter 'another'
		console.log("Incrementing counter 'another'...");
		const response3 = await fetch(`${baseUrl}/increment/another`, {
			method: "POST",
		});
		const result3 = await response3.text();
		console.log(result3);

		console.log("✅ Demo completed!");
	} catch (error) {
		console.error("❌ Error:", error);
		process.exit(1);
	}
}

main().catch(console.error);
