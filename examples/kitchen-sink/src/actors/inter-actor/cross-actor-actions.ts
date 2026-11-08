import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export interface InventoryInput {
	initialStock: number;
	itemName: string;
}

export interface CheckoutInput {
	customerName: string;
}

export interface CheckoutItem {
	itemId: string;
	itemName: string;
	quantity: number;
}

export interface CheckoutResult {
	success: boolean;
	message: string;
	remainingStock?: number;
}

// Inventory actor manages stock for a specific item
export const inventory = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS inventory (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					item_name TEXT NOT NULL,
					stock INTEGER NOT NULL
				);
			`);
			// Track which checkouts have reserved items
			await db.execute(`
				CREATE TABLE IF NOT EXISTS reservations (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					checkout_id TEXT NOT NULL
				);
			`);
		},
	}),

	// Each item has its own inventory actor instance
	onCreate: async (c, input?: InventoryInput) => {
		await c.db.execute(
			"INSERT OR IGNORE INTO inventory (id, item_name, stock) VALUES (1, ?, ?)",
			input?.itemName ?? "Widget",
			input?.initialStock ?? 100,
		);
	},

	actions: {
		// Check current stock
		getStock: async (c) => {
			const item = await loadInventory(c.db);
			return {
				itemName: item.item_name,
				stock: item.stock,
			};
		},

		// Reserve items for checkout (called by checkout actor)
		reserveItems: async (c, checkoutId: string, quantity: number) => {
			const remainingStock = await c.db.transaction(
				async (tx) => {
					// Take the stock only if enough remains, so concurrent
					// reservations can never oversell.
					const [item] = await tx.execute<{ stock: number }>(
						"UPDATE inventory SET stock = stock - ? WHERE id = 1 AND stock >= ? RETURNING stock",
						quantity,
						quantity,
					);
					if (!item) return undefined;

					await tx.execute(
						"INSERT INTO reservations (checkout_id) VALUES (?)",
						checkoutId,
					);
					return item.stock;
				},
				{ name: "reserve-items" },
			);

			if (remainingStock === undefined) {
				const item = await loadInventory(c.db);
				return {
					success: false,
					message: `Insufficient stock. Available: ${item.stock}, Requested: ${quantity}`,
					availableStock: item.stock,
				};
			}

			return {
				success: true,
				message: `Reserved ${quantity} items for checkout ${checkoutId}`,
				remainingStock,
			};
		},

		// Release reserved items if checkout is cancelled
		releaseItems: async (c, checkoutId: string, quantity: number) => {
			const remainingStock = await c.db.transaction(
				async (tx) => {
					const released = await tx.execute<{ id: number }>(
						"DELETE FROM reservations WHERE id = (SELECT id FROM reservations WHERE checkout_id = ? ORDER BY id LIMIT 1) RETURNING id",
						checkoutId,
					);
					if (released.length > 0) {
						await tx.execute(
							"UPDATE inventory SET stock = stock + ? WHERE id = 1",
							quantity,
						);
					}
					const item = await loadInventory(tx);
					return item.stock;
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
				);
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS checkout_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					item_id TEXT NOT NULL,
					item_name TEXT NOT NULL,
					quantity INTEGER NOT NULL
				);
			`);
		},
	}),

	onCreate: async (c, input?: CheckoutInput) => {
		await c.db.execute(
			"INSERT OR IGNORE INTO checkout (id, customer_name, completed) VALUES (1, ?, 0)",
			input?.customerName ?? "Guest",
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
				"INSERT INTO checkout_items (item_id, item_name, quantity) VALUES (?, ?, ?)",
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
			const summary = await loadCheckout(c.db);
			const items = await loadCheckoutItems(c.db);
			return {
				customerName: summary.customer_name,
				items,
				completed: summary.completed === 1,
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
			// Release all reserved items
			for (const item of await loadCheckoutItems(c.db)) {
				const inventoryActor = c
					.client()
					.inventory.getOrCreate([item.itemId]);
				await inventoryActor.releaseItems(c.actorId, item.quantity);
			}

			// Clear the cart
			await c.db.execute("DELETE FROM checkout_items");

			return {
				success: true,
				message: "Checkout cancelled, items returned to inventory",
			};
		},
	},
});

async function loadInventory(
	database: RawAccess,
): Promise<{ item_name: string; stock: number }> {
	const rows = await database.execute<{ item_name: string; stock: number }>(
		"SELECT item_name, stock FROM inventory WHERE id = 1",
	);
	if (!rows[0])
		throw new Error("inventory row missing; onCreate did not run");
	return rows[0];
}

async function loadCheckout(
	database: RawAccess,
): Promise<{ customer_name: string; completed: number }> {
	const rows = await database.execute<{
		customer_name: string;
		completed: number;
	}>("SELECT customer_name, completed FROM checkout WHERE id = 1");
	if (!rows[0]) throw new Error("checkout row missing; onCreate did not run");
	return rows[0];
}

async function loadCheckoutItems(database: RawAccess): Promise<CheckoutItem[]> {
	const rows = await database.execute<{
		item_id: string;
		item_name: string;
		quantity: number;
	}>("SELECT item_id, item_name, quantity FROM checkout_items ORDER BY id");
	return rows.map((row) => ({
		itemId: row.item_id,
		itemName: row.item_name,
		quantity: row.quantity,
	}));
}
