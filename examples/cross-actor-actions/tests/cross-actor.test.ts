import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/backend/registry";

describe("cross-actor communication", () => {
	test("checkout reserves items from inventory", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create inventory with 10 laptops
		const inventory = await client.inventory.create(["laptop"], {
			input: {
				itemName: "Laptop",
				initialStock: 10,
			},
		});

		// Verify initial stock
		let stock = await inventory.getStock();
		expect(stock.stock).toBe(10);

		// Create checkout
		const checkout = await client.checkout.create(["checkout-1"], {
			input: {
				customerName: "Alice",
			},
		});

		// Add item to checkout (this calls inventory actor)
		const result = await checkout.addItem("laptop", 3);

		expect(result.success).toBe(true);
		expect(result.message).toContain("Added 3 Laptop");
		expect(result.remainingStock).toBe(7);

		// Verify inventory was updated
		stock = await inventory.getStock();
		expect(stock.stock).toBe(7);

		// Verify checkout has the items
		const summary = await checkout.getSummary();
		expect(summary.items).toHaveLength(1);
		expect(summary.items[0]).toMatchObject({
			itemId: "laptop",
			itemName: "Laptop",
			quantity: 3,
		});
		expect(summary.totalItems).toBe(3);
	});

	test("insufficient inventory prevents reservation", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create inventory with only 5 phones
		await client.inventory.create(["phone"], {
			input: {
				itemName: "Phone",
				initialStock: 5,
			},
		});

		// Create checkout
		const checkout = await client.checkout.create(["checkout-2"], {
			input: {
				customerName: "Bob",
			},
		});

		// Try to add more items than available
		const result = await checkout.addItem("phone", 10);

		expect(result.success).toBe(false);
		expect(result.message).toContain("Insufficient stock");

		// Verify checkout is empty
		const summary = await checkout.getSummary();
		expect(summary.items).toHaveLength(0);
	});

	test("cancel checkout releases reserved items", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create inventory
		const inventory = await client.inventory.create(["laptop-cancel"], {
			input: {
				itemName: "Laptop",
				initialStock: 20,
			},
		});

		// Create checkout and add items
		const checkout = await client.checkout.create(["checkout-cancel"], {
			input: {
				customerName: "Charlie",
			},
		});

		await checkout.addItem("laptop-cancel", 5);

		// Verify stock decreased
		let stock = await inventory.getStock();
		expect(stock.stock).toBe(15);

		// Cancel checkout
		const cancelResult = await checkout.cancelCheckout();
		expect(cancelResult.success).toBe(true);

		// Verify items returned to inventory
		stock = await inventory.getStock();
		expect(stock.stock).toBe(20);

		// Verify checkout is empty
		const summary = await checkout.getSummary();
		expect(summary.items).toHaveLength(0);
	});

	test("multiple checkouts can reserve from same inventory", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create inventory with 50 items
		const inventory = await client.inventory.create(["shared-item"], {
			input: {
				itemName: "Shared Item",
				initialStock: 50,
			},
		});

		// Create two checkouts
		const checkout1 = await client.checkout.create(["checkout-multi-1"], {
			input: {
				customerName: "Customer 1",
			},
		});

		const checkout2 = await client.checkout.create(["checkout-multi-2"], {
			input: {
				customerName: "Customer 2",
			},
		});

		// Both checkouts add items
		await checkout1.addItem("shared-item", 20);
		await checkout2.addItem("shared-item", 15);

		// Verify total stock decreased correctly
		const stock = await inventory.getStock();
		expect(stock.stock).toBe(15); // 50 - 20 - 15 = 15

		// Verify each checkout has their items
		const summary1 = await checkout1.getSummary();
		const summary2 = await checkout2.getSummary();

		expect(summary1.totalItems).toBe(20);
		expect(summary2.totalItems).toBe(15);
	});

	test("complete checkout keeps reservation", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create inventory
		const inventory = await client.inventory.create(["laptop-complete"], {
			input: {
				itemName: "Laptop",
				initialStock: 30,
			},
		});

		// Create checkout
		const checkout = await client.checkout.create(["checkout-complete"], {
			input: {
				customerName: "Dave",
			},
		});

		// Add items
		await checkout.addItem("laptop-complete", 10);

		// Verify stock decreased
		let stock = await inventory.getStock();
		expect(stock.stock).toBe(20);

		// Complete checkout
		const completeResult = await checkout.completeCheckout();
		expect(completeResult.success).toBe(true);

		// Verify stock is still decreased (items are purchased)
		stock = await inventory.getStock();
		expect(stock.stock).toBe(20);

		// Verify checkout is marked as completed
		const summary = await checkout.getSummary();
		expect(summary.completed).toBe(true);
		expect(summary.totalItems).toBe(10);
	});

	test("checkout with multiple item types", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create multiple inventories
		await client.inventory.create(["laptop-multi"], {
			input: {
				itemName: "Laptop",
				initialStock: 10,
			},
		});

		await client.inventory.create(["phone-multi"], {
			input: {
				itemName: "Phone",
				initialStock: 20,
			},
		});

		// Create checkout
		const checkout = await client.checkout.create(
			["checkout-multi-items"],
			{
				input: {
					customerName: "Eve",
				},
			},
		);

		// Add multiple item types
		await checkout.addItem("laptop-multi", 2);
		await checkout.addItem("phone-multi", 5);

		// Verify checkout summary
		const summary = await checkout.getSummary();
		expect(summary.items).toHaveLength(2);
		expect(summary.totalItems).toBe(7); // 2 + 5

		// Find laptop and phone items
		const laptopItem = summary.items.find(
			(i) => i.itemId === "laptop-multi",
		);
		const phoneItem = summary.items.find((i) => i.itemId === "phone-multi");

		expect(laptopItem).toMatchObject({
			itemName: "Laptop",
			quantity: 2,
		});

		expect(phoneItem).toMatchObject({
			itemName: "Phone",
			quantity: 5,
		});
	});

	test("release items for specific checkout", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create inventory
		const inventory = await client.inventory.create(["laptop-release"], {
			input: {
				itemName: "Laptop",
				initialStock: 100,
			},
		});

		// Create two checkouts
		const checkout1 = await client.checkout.create(["checkout-release-1"], {
			input: {
				customerName: "Customer 1",
			},
		});

		const checkout2 = await client.checkout.create(["checkout-release-2"], {
			input: {
				customerName: "Customer 2",
			},
		});

		// Both reserve items
		await checkout1.addItem("laptop-release", 30);
		await checkout2.addItem("laptop-release", 40);

		// Stock should be 30 (100 - 30 - 40)
		let stock = await inventory.getStock();
		expect(stock.stock).toBe(30);

		// Cancel only checkout1
		await checkout1.cancelCheckout();

		// Stock should increase by 30
		stock = await inventory.getStock();
		expect(stock.stock).toBe(60);

		// checkout2 should still have its items
		const summary2 = await checkout2.getSummary();
		expect(summary2.totalItems).toBe(40);
	});
});
