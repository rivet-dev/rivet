import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export interface InventoryInput {
	initialStock: number;
	itemName: string;
}

export interface CheckoutInput {
	customerName: string;
}

export type CheckoutItem = {
	itemId: string;
	itemName: string;
	quantity: number;
};

export interface CheckoutResult {
	success: boolean;
	message: string;
	remainingStock?: number;
}

// Inventory actor manages stock for a specific item
export const inventory = actor({
	// Each item has its own inventory actor instance with its own SQLite database:
	// https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS inventory (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					item_name TEXT NOT NULL,
					stock INTEGER NOT NULL
				)
			`);
			// Track which checkouts have reserved items
			await db.execute(`
				CREATE TABLE IF NOT EXISTS reservations (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					checkout_id TEXT NOT NULL
				)
			`);
		},
	}),

	// Save the item from the creation input once, when the actor is created
	onCreate: async (c, input: InventoryInput) => {
		await c.db.execute(
			"INSERT INTO inventory (id, item_name, stock) VALUES (1, ?, ?)",
			input.itemName,
			input.initialStock,
		);
	},

	actions: {
		// Check current stock
		getStock: async (c) => {
			const [row] = (await c.db.execute(
				"SELECT item_name AS itemName, stock FROM inventory WHERE id = 1",
			)) as { itemName: string; stock: number }[];
			return row;
		},

		// Reserve items for checkout (called by checkout actor)
		reserveItems: async (c, checkoutId: string, quantity: number) => {
			return await c.db.transaction(
				async (tx) => {
					// Only take the stock if enough is left, in a single statement
					const [updated] = await tx.execute<{ stock: number }>(
						"UPDATE inventory SET stock = stock - ? WHERE id = 1 AND stock >= ? RETURNING stock",
						quantity,
						quantity,
					);
					if (!updated) {
						const [{ stock }] = await tx.execute<{ stock: number }>(
							"SELECT stock FROM inventory WHERE id = 1",
						);
						return {
							success: false,
							message: `Insufficient stock. Available: ${stock}, Requested: ${quantity}`,
							availableStock: stock,
						};
					}

					// Record the reservation in the same transaction as the stock change
					await tx.execute(
						"INSERT INTO reservations (checkout_id) VALUES (?)",
						checkoutId,
					);

					return {
						success: true,
						message: `Reserved ${quantity} items for checkout ${checkoutId}`,
						remainingStock: updated.stock,
					};
				},
				{ name: "reserve-items" },
			);
		},

		// Release reserved items if checkout is cancelled
		releaseItems: async (c, checkoutId: string, quantity: number) => {
			const remainingStock = await c.db.transaction(
				async (tx) => {
					const released = await tx.execute(
						"DELETE FROM reservations WHERE seq = (SELECT seq FROM reservations WHERE checkout_id = ? LIMIT 1) RETURNING seq",
						checkoutId,
					);
					if (released.length > 0) {
						await tx.execute(
							"UPDATE inventory SET stock = stock + ? WHERE id = 1",
							quantity,
						);
					}
					const [{ stock }] = await tx.execute<{ stock: number }>(
						"SELECT stock FROM inventory WHERE id = 1",
					);
					return stock;
				},
				{ name: "release-items" },
			);
			return {
				success: true,
				remainingStock,
			};
		},
	},
});

// Checkout actor manages the checkout process and communicates with inventory
export const checkout = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS checkout (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					customer_name TEXT NOT NULL,
					completed INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS items (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					item_id TEXT NOT NULL,
					item_name TEXT NOT NULL,
					quantity INTEGER NOT NULL
				)
			`);
		},
	}),

	// Save the customer from the creation input once, when the actor is created
	onCreate: async (c, input: CheckoutInput) => {
		await c.db.execute(
			"INSERT INTO checkout (id, customer_name, completed) VALUES (1, ?, 0)",
			input.customerName,
		);
	},

	actions: {
		// Add item to checkout and reserve from inventory
		addItem: async (
			c,
			itemId: string,
			quantity: number,
		): Promise<CheckoutResult> => {
			// Use server-side client to communicate with inventory actor
			// https://rivet.dev/docs/actors/communicating-between-actors
			const inventoryActor = c.client().inventory.getOrCreate([itemId]);

			// Get item details
			const itemInfo = await inventoryActor.getStock();

			// Try to reserve items from inventory
			const reservation = await inventoryActor.reserveItems(
				c.actorId, // Use checkout ID as reservation ID
				quantity,
			);

			if (!reservation.success) {
				return {
					success: false,
					message: reservation.message,
				};
			}

			// Add item to checkout
			await c.db.execute(
				"INSERT INTO items (item_id, item_name, quantity) VALUES (?, ?, ?)",
				itemId,
				itemInfo.itemName,
				quantity,
			);

			return {
				success: true,
				message: `Added ${quantity} ${itemInfo.itemName} to checkout`,
				remainingStock: reservation.remainingStock,
			};
		},

		// Get checkout summary
		getSummary: async (c) => {
			const [row] = (await c.db.execute(
				"SELECT customer_name, completed FROM checkout WHERE id = 1",
			)) as { customer_name: string; completed: number }[];
			const items = await c.db.execute<CheckoutItem>(
				"SELECT item_id AS itemId, item_name AS itemName, quantity FROM items ORDER BY seq",
			);
			return {
				customerName: row.customer_name,
				items,
				completed: row.completed === 1,
				totalItems: items.reduce((sum, item) => sum + item.quantity, 0),
			};
		},

		// Complete the checkout
		completeCheckout: async (c) => {
			await c.db.execute(
				"UPDATE checkout SET completed = 1 WHERE id = 1",
			);
			return {
				success: true,
				message: "Checkout completed successfully",
			};
		},

		// Cancel checkout and release all reservations
		cancelCheckout: async (c) => {
			const items = await c.db.execute<{
				seq: number;
				itemId: string;
				quantity: number;
			}>(
				"SELECT seq, item_id AS itemId, quantity FROM items ORDER BY seq",
			);

			// Release each reserved item, then remove only that item from the
			// cart. Items added while this runs stay in the cart with their
			// reservations intact.
			for (const item of items) {
				const inventoryActor = c
					.client()
					.inventory.getOrCreate([item.itemId]);
				await inventoryActor.releaseItems(c.actorId, item.quantity);
				await c.db.execute("DELETE FROM items WHERE seq = ?", item.seq);
			}

			return {
				success: true,
				message: "Checkout cancelled, items returned to inventory",
			};
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { inventory, checkout },
});

// Start the server on port 6420
registry.start();
