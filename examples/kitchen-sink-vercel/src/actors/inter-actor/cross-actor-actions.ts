import { actor } from "rivetkit";

export interface InventoryInput {
	initialStock: number;
	itemName: string;
}

export interface InventoryState {
	itemName: string;
	stock: number;
	reservations: string[]; // Track which checkouts have reserved items
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

export interface CheckoutState {
	customerName: string;
	items: CheckoutItem[];
	completed: boolean;
}

// Inventory actor manages stock for a specific item
export const inventory = actor({
	// Each item has its own inventory actor instance
	createState: (_c, input?: InventoryInput): InventoryState => ({
		itemName: input?.itemName ?? "Widget",
		stock: input?.initialStock ?? 100,
		reservations: [],
	}),

	actions: {
		// Check current stock
		getStock: (c) => ({
			itemName: c.state.itemName,
			stock: c.state.stock,
		}),

		// Reserve items for checkout (called by checkout actor)
		reserveItems: (c, checkoutId: string, quantity: number) => {
			if (c.state.stock < quantity) {
				return {
					success: false,
					message: `Insufficient stock. Available: ${c.state.stock}, Requested: ${quantity}`,
					availableStock: c.state.stock,
				};
			}

			// Reserve the items
			c.state.stock -= quantity;
			c.state.reservations.push(checkoutId);

			return {
				success: true,
				message: `Reserved ${quantity} items for checkout ${checkoutId}`,
				remainingStock: c.state.stock,
			};
		},

		// Release reserved items if checkout is cancelled
		releaseItems: (c, checkoutId: string, quantity: number) => {
			const index = c.state.reservations.indexOf(checkoutId);
			if (index > -1) {
				c.state.reservations.splice(index, 1);
				c.state.stock += quantity;
			}
			return {
				success: true,
				remainingStock: c.state.stock,
			};
		},
	},
});

// Checkout actor manages the checkout process and communicates with inventory
export const checkout = actor({
	createState: (_c, input?: CheckoutInput): CheckoutState => ({
		customerName: input?.customerName ?? "Guest",
		items: [],
		completed: false,
	}),

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
			c.state.items.push({
				itemId,
				itemName: itemInfo.itemName,
				quantity,
			});

			return {
				success: true,
				message: `Added ${quantity} ${itemInfo.itemName} to checkout`,
				remainingStock: reservation.remainingStock,
			};
		},

		// Get checkout summary
		getSummary: (c) => ({
			customerName: c.state.customerName,
			items: c.state.items,
			completed: c.state.completed,
			totalItems: c.state.items.reduce(
				(sum, item) => sum + item.quantity,
				0,
			),
		}),

		// Complete the checkout
		completeCheckout: (c) => {
			c.state.completed = true;
			return {
				success: true,
				message: "Checkout completed successfully",
			};
		},

		// Cancel checkout and release all reservations
		cancelCheckout: async (c) => {
			// Release all reserved items
			for (const item of c.state.items) {
				const inventoryActor = c
					.client()
					.inventory.getOrCreate([item.itemId]);
				await inventoryActor.releaseItems(c.actorId, item.quantity);
			}

			// Clear the cart
			c.state.items = [];

			return {
				success: true,
				message: "Checkout cancelled, items returned to inventory",
			};
		},
	},
});
