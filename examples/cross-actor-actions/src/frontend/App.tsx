import { createClient } from "rivetkit/client";
import { useEffect, useState } from "react";
import type { registry } from "../backend/registry";

const client = createClient<typeof registry>("http://localhost:6420");

interface ItemStock {
	itemName: string;
	stock: number;
}

interface CheckoutSummary {
	customerName: string;
	items: Array<{
		itemId: string;
		itemName: string;
		quantity: number;
	}>;
	completed: boolean;
	totalItems: number;
}

export function App() {
	const [customerName, setCustomerName] = useState("Customer");
	const [selectedItem, setSelectedItem] = useState("laptop");
	const [quantity, setQuantity] = useState(1);
	const [message, setMessage] = useState("");

	const [laptopStock, setLaptopStock] = useState<ItemStock | null>(null);
	const [phoneStock, setPhoneStock] = useState<ItemStock | null>(null);
	const [checkoutSummary, setCheckoutSummary] = useState<CheckoutSummary | null>(
		null
	);

	const initializeInventory = async () => {
		// Initialize laptop inventory
		const laptop = await client.inventory.create(["laptop"], {
			input: {
				itemName: "Laptop",
				initialStock: 10,
			},
		});
		const laptopInfo = await laptop.getStock();
		setLaptopStock(laptopInfo);

		// Initialize phone inventory
		const phone = await client.inventory.create(["phone"], {
			input: {
				itemName: "Phone",
				initialStock: 20,
			},
		});
		const phoneInfo = await phone.getStock();
		setPhoneStock(phoneInfo);

		setMessage("Inventory initialized");
	};

	const refreshInventory = async () => {
		const laptopInventory = client.inventory.get(["laptop"]);
		const laptopInfo = await laptopInventory.getStock();
		setLaptopStock(laptopInfo);

		const phoneInventory = client.inventory.get(["phone"]);
		const phoneInfo = await phoneInventory.getStock();
		setPhoneStock(phoneInfo);
	};

	const createCheckout = async () => {
		const checkout = await client.checkout.create(["session-1"], {
			input: {
				customerName,
			},
		});

		const summary = await checkout.getSummary();
		setCheckoutSummary(summary);
		setMessage("Checkout session created");
	};

	const addItemToCheckout = async () => {
		const checkout = client.checkout.get(["session-1"]);
		const result = await checkout.addItem(selectedItem, quantity);

		if (result.success) {
			setMessage(result.message);
			// Refresh checkout and inventory
			const summary = await checkout.getSummary();
			setCheckoutSummary(summary);
			await refreshInventory();
		} else {
			setMessage(`Error: ${result.message}`);
		}
	};

	const completeCheckout = async () => {
		const checkout = client.checkout.get(["session-1"]);
		const result = await checkout.completeCheckout();
		setMessage(result.message);

		const summary = await checkout.getSummary();
		setCheckoutSummary(summary);
	};

	const cancelCheckout = async () => {
		const checkout = client.checkout.get(["session-1"]);
		const result = await checkout.cancelCheckout();
		setMessage(result.message);

		const summary = await checkout.getSummary();
		setCheckoutSummary(summary);
		await refreshInventory();
	};

	return (
		<div className="container">
			<div className="header">
				<h1>Quickstart: Cross-Actor Actions</h1>
				<p>
					Demonstrates actors communicating with each other - checkout calling
					inventory
				</p>
			</div>

			{message && <div className="message">{message}</div>}

			<div className="grid">
				{/* Inventory Section */}
				<div className="section">
					<h2>Inventory Management</h2>
					<button onClick={initializeInventory} className="primary">
						Initialize Inventory
					</button>

					{laptopStock && (
						<div className="inventory-item">
							<h3>{laptopStock.itemName}</h3>
							<p>Stock: {laptopStock.stock} units</p>
						</div>
					)}

					{phoneStock && (
						<div className="inventory-item">
							<h3>{phoneStock.itemName}</h3>
							<p>Stock: {phoneStock.stock} units</p>
						</div>
					)}
				</div>

				{/* Checkout Section */}
				<div className="section">
					<h2>Checkout</h2>

					<div className="form-group">
						<label>Customer Name:</label>
						<input
							type="text"
							value={customerName}
							onChange={(e) => setCustomerName(e.target.value)}
							placeholder="Enter customer name"
						/>
					</div>

					<button
						onClick={createCheckout}
						className="primary"
					>
						Create Checkout
					</button>

					{checkoutSummary && !checkoutSummary.completed && (
						<div className="checkout-section">
							<h3>Add Items</h3>
							<div className="form-group">
								<label>Item:</label>
								<select
									value={selectedItem}
									onChange={(e) => setSelectedItem(e.target.value)}
								>
									<option value="laptop">Laptop</option>
									<option value="phone">Phone</option>
								</select>
							</div>

							<div className="form-group">
								<label>Quantity:</label>
								<input
									type="number"
									value={quantity}
									onChange={(e) => setQuantity(Number(e.target.value))}
									min="1"
								/>
							</div>

							<button onClick={addItemToCheckout} className="secondary">
								Add to Checkout
							</button>
						</div>
					)}

					{checkoutSummary && (
						<div className="summary">
							<h3>Checkout Summary</h3>
							<p>
								<strong>Customer:</strong> {checkoutSummary.customerName}
							</p>
							<p>
								<strong>Status:</strong>{" "}
								{checkoutSummary.completed ? "Completed" : "In Progress"}
							</p>
							<p>
								<strong>Total Items:</strong> {checkoutSummary.totalItems}
							</p>

							{checkoutSummary.items.length > 0 && (
								<div className="items-list">
									<h4>Items:</h4>
									{checkoutSummary.items.map((item, idx) => (
										<div key={idx} className="item">
											{item.quantity}x {item.itemName}
										</div>
									))}
								</div>
							)}

							{!checkoutSummary.completed && (
								<div className="button-group">
									<button onClick={completeCheckout} className="success">
										Complete Checkout
									</button>
									<button onClick={cancelCheckout} className="danger">
										Cancel Checkout
									</button>
								</div>
							)}
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
