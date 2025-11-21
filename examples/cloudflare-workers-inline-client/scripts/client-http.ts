const baseUrl = process.env.BASE_URL ?? "http://localhost:8787";

async function main() {
	console.log("🚀 Cloudflare Workers Client Demo");

	try {
		for (let i = 0; i < 3; i++) {
			// Increment counter
			console.log("Incrementing counter...");
			const response = await fetch(`${baseUrl}/increment/demo`, {
				method: "POST",
			});
			const result = await response.text();
			console.log(result);
		}

		console.log("✅ Demo completed!");
	} catch (error) {
		console.error("❌ Error:", error);
		process.exit(1);
	}
}

main().catch(console.error);
