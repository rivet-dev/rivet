// ORDER PROCESSOR (Steps Demo)
// Demonstrates: Sequential workflow steps with automatic retries
// One actor per order - actor key is the order ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";

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
	createState: (c): Order => ({
		id: c.actorKey[0] as string,
		status: "pending",
		step: 0,
		createdAt: Date.now(),
	}),
	events: {
		orderUpdated: event<Order>(),
	},

	actions: {
		getOrder: (c): Order => c.state,
	},

	run: workflow(async (ctx) => {
		await ctx.loop("process-order", async (loopCtx) => {
			await loopCtx.step("validate", async (step) => {
				step.log.info({
					msg: "processing order",
					orderId: step.state.id,
				});
				step.state.status = "validating";
				step.state.step = 1;
				step.broadcast("orderUpdated", step.state);
				await simulateWork("validation", 0.05);
			});

			await loopCtx.step("charge", async (step) => {
				step.state.status = "charging";
				step.state.step = 2;
				step.broadcast("orderUpdated", step.state);
				await simulateWork("payment", 0.1);
			});

			await loopCtx.step("fulfill", async (step) => {
				step.state.status = "fulfilling";
				step.state.step = 3;
				step.broadcast("orderUpdated", step.state);
				await simulateWork("fulfillment", 0.05);
			});

			await loopCtx.step("complete", async (step) => {
				step.state.status = "completed";
				step.state.step = 4;
				step.state.completedAt = Date.now();
				step.broadcast("orderUpdated", step.state);
				step.log.info({
					msg: "order completed",
					orderId: step.state.id,
				});
			});

			return Loop.break(undefined);
		});
	}),
});
