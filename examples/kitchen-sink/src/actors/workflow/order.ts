// ORDER PROCESSOR (Steps Demo)
// Demonstrates: Sequential workflow steps with automatic retries
// One actor per order - actor key is the order ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

export type OrderStatus =
	| "pending"
	| "validating"
	| "charging"
	| "fulfilling"
	| "completed"
	| "failed";

export type Order = {
	id: string;
	status: OrderStatus;
	step: number;
	error?: string;
	createdAt: number;
	completedAt?: number;
};

async function simulateWork(name: string, failChance = 0.1): Promise<void> {
	await new Promise((resolve) =>
		setTimeout(resolve, 500 + Math.random() * 1000),
	);
	if (Math.random() < failChance) {
		throw new Error(`${name} failed (simulated)`);
	}
}

export const order = actor({
	db: recordDb(),
	onCreate: async (c) => {
		await saveRecord<Order>(c.db, {
			id: c.key[0] as string,
			status: "pending",
			step: 0,
			createdAt: Date.now(),
		});
	},
	events: {
		orderUpdated: event<Order>(),
	},

	actions: {
		getOrder: (c): Promise<Order> => loadRecord<Order>(c.db),
	},

	run: workflow(async (ctx) => {
		await ctx.loop("process-order", async (loopCtx) => {
			await loopCtx.step("validate", async (c) => {
				const updated = await updateRecord<Order>(c.db, (order) => {
					order.status = "validating";
					order.step = 1;
				});
				c.log.info({
					msg: "processing order",
					orderId: updated.id,
				});
				c.broadcast("orderUpdated", updated);
				await simulateWork("validation", 0.05);
			});

			await loopCtx.step("charge", async (c) => {
				const updated = await updateRecord<Order>(c.db, (order) => {
					order.status = "charging";
					order.step = 2;
				});
				c.broadcast("orderUpdated", updated);
				await simulateWork("payment", 0.1);
			});

			await loopCtx.step("fulfill", async (c) => {
				const updated = await updateRecord<Order>(c.db, (order) => {
					order.status = "fulfilling";
					order.step = 3;
				});
				c.broadcast("orderUpdated", updated);
				await simulateWork("fulfillment", 0.05);
			});

			await loopCtx.step("complete", async (c) => {
				const updated = await updateRecord<Order>(c.db, (order) => {
					order.status = "completed";
					order.step = 4;
					order.completedAt = Date.now();
				});
				c.broadcast("orderUpdated", updated);
				c.log.info({
					msg: "order completed",
					orderId: updated.id,
				});
			});

			return Loop.break(undefined);
		});
	}),
});
