// ORDER PROCESSOR (Steps Demo)
// Demonstrates: Sequential workflow steps with automatic retries
// One actor per order - actor key is the order ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

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

type State = Order;

async function simulateWork(name: string, failChance = 0.1): Promise<void> {
	await new Promise((resolve) =>
		setTimeout(resolve, 500 + Math.random() * 1000)
	);
	if (Math.random() < failChance) {
		throw new Error(`${name} failed (simulated)`);
	}
}

export const order = actor({
	createState: (c): Order => ({
		id: c.key[0] as string,
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
				const c = actorCtx<State>(loopCtx);

				await loopCtx.step("validate", async () => {
					ctx.log.info({ msg: "processing order", orderId: c.state.id });
					c.state.status = "validating";
					c.state.step = 1;
					c.broadcast("orderUpdated", c.state);
					await simulateWork("validation", 0.05);
				});

				await loopCtx.step("charge", async () => {
					c.state.status = "charging";
					c.state.step = 2;
					c.broadcast("orderUpdated", c.state);
					await simulateWork("payment", 0.1);
				});

				await loopCtx.step("fulfill", async () => {
					c.state.status = "fulfilling";
					c.state.step = 3;
					c.broadcast("orderUpdated", c.state);
					await simulateWork("fulfillment", 0.05);
				});

				await loopCtx.step("complete", async () => {
					c.state.status = "completed";
					c.state.step = 4;
					c.state.completedAt = Date.now();
					c.broadcast("orderUpdated", c.state);
					ctx.log.info({ msg: "order completed", orderId: c.state.id });
				});

				return Loop.break(undefined);
			});
	}),
});
