// PAYMENT PROCESSOR (Rollback Demo)
// Demonstrates: Rollback checkpoints with compensating actions
// One actor per transaction - actor key is the transaction ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

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

export type TransactionInput = {
	amount?: number;
	shouldFail?: boolean;
};

export const payment = actor({
	db: recordDb(),
	onCreate: async (c, input?: TransactionInput) => {
		await saveRecord<Transaction>(c.db, {
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
		});
	},
	events: {
		transactionStarted: event<Transaction>(),
		transactionUpdated: event<Transaction>(),
		transactionCompleted: event<Transaction>(),
	},

	actions: {
		getTransaction: (c): Promise<Transaction> =>
			loadRecord<Transaction>(c.db),
	},

	run: workflow(async (ctx) => {
		await ctx.loop("payment-loop", async (loopCtx) => {
			await loopCtx.step("init-payment", async (c) => {
				const transaction = await loadRecord<Transaction>(c.db);
				c.log.info({
					msg: "starting payment processing",
					txId: transaction.id,
					amount: transaction.amount,
					shouldFail: transaction.shouldFail,
				});
				c.broadcast("transactionStarted", transaction);
			});

			await loopCtx.rollbackCheckpoint("payment-checkpoint");

			// Step 1: Reserve inventory
			await loopCtx.step({
				name: "reserve-inventory",
				run: async (c) => {
					const updated = await updateRecord<Transaction>(
						c.db,
						(transaction) => {
							transaction.status = "reserving";
							const step = transaction.steps.find(
								(s) => s.name === "reserve-inventory",
							);
							if (step) {
								step.status = "completed";
								step.completedAt = Date.now();
							}
						},
					);
					c.broadcast("transactionUpdated", updated);

					await new Promise((r) =>
						setTimeout(r, 500 + Math.random() * 500),
					);
					c.log.info({
						msg: "inventory reserved",
						txId: updated.id,
					});
					return { reserved: true };
				},
				rollback: async (c) => {
					// Set rolling_back status on first rollback
					const updated = await updateRecord<Transaction>(
						c.db,
						(transaction) => {
							transaction.status = "rolling_back";
							const step = transaction.steps.find(
								(s) => s.name === "reserve-inventory",
							);
							if (step) {
								step.status = "rolled_back";
								step.rolledBackAt = Date.now();
							}
						},
					);
					c.log.info({
						msg: "inventory released",
						txId: updated.id,
					});
					c.broadcast("transactionUpdated", updated);
					// Small delay so UI can show the rollback
					await new Promise((r) => setTimeout(r, 400));
				},
			});

			// Step 2: Charge card
			await loopCtx.step({
				name: "charge-card",
				run: async (c) => {
					const updated = await updateRecord<Transaction>(
						c.db,
						(transaction) => {
							transaction.status = "charging";
							const step = transaction.steps.find(
								(s) => s.name === "charge-card",
							);
							if (step) {
								step.status = "completed";
								step.completedAt = Date.now();
							}
						},
					);
					c.broadcast("transactionUpdated", updated);

					await new Promise((r) =>
						setTimeout(r, 500 + Math.random() * 500),
					);

					if (updated.shouldFail) {
						throw new Error("Payment declined (simulated)");
					}

					c.log.info({ msg: "card charged", txId: updated.id });
					return { chargeId: `ch_${updated.id}` };
				},
				rollback: async (c) => {
					const updated = await updateRecord<Transaction>(
						c.db,
						(transaction) => {
							transaction.status = "rolling_back";
							const step = transaction.steps.find(
								(s) => s.name === "charge-card",
							);
							if (step) {
								step.status = "rolled_back";
								step.rolledBackAt = Date.now();
							}
						},
					);
					c.log.info({ msg: "charge refunded", txId: updated.id });
					c.broadcast("transactionUpdated", updated);
					// Small delay so UI can show the rollback
					await new Promise((r) => setTimeout(r, 400));
				},
			});

			// Step 3: Complete order
			await loopCtx.step("complete-order", async (c) => {
				const completing = await updateRecord<Transaction>(
					c.db,
					(transaction) => {
						transaction.status = "completing";
						const step = transaction.steps.find(
							(s) => s.name === "complete-order",
						);
						if (step) step.status = "completed";
					},
				);
				c.broadcast("transactionUpdated", completing);

				await new Promise((r) =>
					setTimeout(r, 300 + Math.random() * 300),
				);

				const completed = await updateRecord<Transaction>(
					c.db,
					(transaction) => {
						transaction.status = "completed";
						transaction.completedAt = Date.now();
					},
				);
				c.log.info({ msg: "order completed", txId: completed.id });
				c.broadcast("transactionCompleted", completed);
			});

			return Loop.break(undefined);
		});
	}),
});
