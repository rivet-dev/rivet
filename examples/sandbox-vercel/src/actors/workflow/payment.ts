// PAYMENT PROCESSOR (Rollback Demo)
// Demonstrates: Rollback checkpoints with compensating actions
// One actor per transaction - actor key is the transaction ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

export type TransactionStep = {
	name: string;
	status: "pending" | "completed" | "rolled_back";
	completedAt?: number;
	rolledBackAt?: number;
};

export type Transaction = {
	id: string;
	amount: number;
	shouldFail: boolean;
	status:
		| "pending"
		| "reserving"
		| "charging"
		| "completing"
		| "completed"
		| "rolling_back"
		| "failed";
	steps: TransactionStep[];
	error?: string;
	startedAt: number;
	completedAt?: number;
};

type State = Transaction;

export type TransactionInput = {
	amount?: number;
	shouldFail?: boolean;
};

export const payment = actor({
	createState: (c, input?: TransactionInput): Transaction => ({
		id: c.key[0] as string,
		amount: input?.amount ?? 100,
		shouldFail: input?.shouldFail ?? false,
		status: "pending",
		steps: [
			{ name: "reserve-inventory", status: "pending" },
			{ name: "charge-card", status: "pending" },
			{ name: "complete-order", status: "pending" },
		],
		startedAt: Date.now(),
	}),
	events: {
		transactionStarted: event<Transaction>(),
		transactionUpdated: event<Transaction>(),
		transactionCompleted: event<Transaction>(),
	},

	actions: {
		getTransaction: (c): Transaction => c.state,
	},

	run: workflow(async (ctx) => {
		await ctx.loop("payment-loop", async (loopCtx) => {
				const c = actorCtx<State>(loopCtx);

				await loopCtx.step("init-payment", async () => {
					ctx.log.info({
						msg: "starting payment processing",
						txId: c.state.id,
						amount: c.state.amount,
						shouldFail: c.state.shouldFail,
					});
					c.broadcast("transactionStarted", c.state);
				});

				await loopCtx.rollbackCheckpoint("payment-checkpoint");

				// Step 1: Reserve inventory
				await loopCtx.step({
					name: "reserve-inventory",
					run: async () => {
						c.state.status = "reserving";
						const step = c.state.steps.find(
							(s) => s.name === "reserve-inventory"
						);
						if (step) {
							step.status = "completed";
							step.completedAt = Date.now();
						}
						c.broadcast("transactionUpdated", c.state);

						await new Promise((r) =>
							setTimeout(r, 500 + Math.random() * 500)
						);
						ctx.log.info({ msg: "inventory reserved", txId: c.state.id });
						return { reserved: true };
					},
					rollback: async () => {
						// Set rolling_back status on first rollback
						c.state.status = "rolling_back";
						const step = c.state.steps.find(
							(s) => s.name === "reserve-inventory"
						);
						if (step) {
							step.status = "rolled_back";
							step.rolledBackAt = Date.now();
						}
						ctx.log.info({ msg: "inventory released", txId: c.state.id });
						c.broadcast("transactionUpdated", c.state);
						// Small delay so UI can show the rollback
						await new Promise((r) => setTimeout(r, 400));
					},
				});

				// Step 2: Charge card
				await loopCtx.step({
					name: "charge-card",
					run: async () => {
						c.state.status = "charging";
						const step = c.state.steps.find((s) => s.name === "charge-card");
						if (step) {
							step.status = "completed";
							step.completedAt = Date.now();
						}
						c.broadcast("transactionUpdated", c.state);

						await new Promise((r) =>
							setTimeout(r, 500 + Math.random() * 500)
						);

						if (c.state.shouldFail) {
							throw new Error("Payment declined (simulated)");
						}

						ctx.log.info({ msg: "card charged", txId: c.state.id });
						return { chargeId: `ch_${c.state.id}` };
					},
					rollback: async () => {
						c.state.status = "rolling_back";
						const step = c.state.steps.find((s) => s.name === "charge-card");
						if (step) {
							step.status = "rolled_back";
							step.rolledBackAt = Date.now();
						}
						ctx.log.info({ msg: "charge refunded", txId: c.state.id });
						c.broadcast("transactionUpdated", c.state);
						// Small delay so UI can show the rollback
						await new Promise((r) => setTimeout(r, 400));
					},
				});

				// Step 3: Complete order
				await loopCtx.step("complete-order", async () => {
						c.state.status = "completing";
						const step = c.state.steps.find((s) => s.name === "complete-order");
						if (step) step.status = "completed";
						c.broadcast("transactionUpdated", c.state);

						await new Promise((r) =>
							setTimeout(r, 300 + Math.random() * 300)
						);

						c.state.status = "completed";
						c.state.completedAt = Date.now();
						ctx.log.info({ msg: "order completed", txId: c.state.id });
						c.broadcast("transactionCompleted", c.state);
					});

				return Loop.break(undefined);
			});
	}),
});
