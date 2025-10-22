export type OrderStatus = "pending" | "packed" | "shipped" | "delivered";

export type OrderInput = {
	customer: string;
};

export type Order = {
	id: string;
	customer: string;
	state: OrderStatus;
	createdAt: number;
	updatedAt: number;
};

export const orderFlow: readonly OrderStatus[] = [
	"pending",
	"packed",
	"shipped",
	"delivered",
];
