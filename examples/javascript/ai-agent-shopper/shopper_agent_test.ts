import { TestClient } from "@rivet-gg/actor-client/test";
import type ShopperAgent from "./shopper_agent.ts";

Deno.test("shopper agent updates and retrieves shopping cart", async () => {
	const client = new TestClient();

	// Get-or-create a shopper agent actor
	const shopperAgent = await client.get<ShopperAgent>({
		name: "shopper_agent",
	});

	// // Test updating the shopping cart
	// await shopperAgent.processPrompt("Please update the shopping cart with 2 items of white paint.");
	// const cart = await shopperAgent.getShoppingCart();
	// assertEquals(cart, [{ slug: "paint-white", count: 2 }]);

	// // Test retrieving the catalog
	// const catalog = await shopperAgent.processPrompt("Can you provide the current catalog?");

	// // Test updating with an invalid item
	// assertThrows(
	// 	async () => {
	// 		await shopperAgent.processPrompt("Please add 1 item with slug 'invalid-slug' to the shopping cart.");
	// 	},
	// 	Error,
	// 	"Item with slug invalid-slug is not in the catalog",
	// );

	// Disconnect from the actor when finished
	await shopperAgent.disconnect();
});
