import assert from "node:assert";
import { TestClient } from "@rivet-gg/actor-client/test";
import type ShopperAgent from "./shopper_agent";

const client = new TestClient();

// Get-or-create a shopper agent actor
const openaiKey = process.env.OPENAI_KEY;
assert.ok(openaiKey);
const shopperAgent = await client.get<ShopperAgent>(
	{
		name: "shopper_agent",
	},
	{
		parameters: {
			openaiKey,
		},
	},
);

// Test updating the shopping cart
await shopperAgent.processPrompt(
	"Please update the shopping cart with 2 items of white paint.",
);
const cart = await shopperAgent.getShoppingCart();
console.log("Cart", cart);
assert.strictEqual(cart.length, 1);
assert.deepStrictEqual(cart[0], { slug: "paint-white", count: 2 });

// Test retrieving the catalog
const catalog = await shopperAgent.processPrompt(
	"Can you provide the current catalog?",
);
console.log("Catalog", catalog);

// Test updating with an invalid item
await assert.rejects(
	async () => {
		await shopperAgent.processPrompt(
			"Please add 1 item with slug 'invalid-slug' to the shopping cart.",
		);
	},
	{
		name: "Error",
		message: "Item with slug invalid-slug is not in the catalog",
	},
);

// Disconnect from the actor when finished
await shopperAgent.disconnect();
