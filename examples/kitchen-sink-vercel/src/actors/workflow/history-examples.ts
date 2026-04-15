import { actor, event, queue } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export type WorkflowHistorySimpleState = {
	id: string;
	status: "pending" | "running" | "completed";
	lastStep?: string;
	startedAt?: number;
	completedAt?: number;
	output?: { success: boolean; processedItems: number };
};

export const workflowHistorySimple = actor({
	createState: (c): WorkflowHistorySimpleState => ({
		id: c.key[0] as string,
		status: "pending",
	}),
	actions: {
		getState: (c): WorkflowHistorySimpleState => c.state,
	},
	run: workflow(async (ctx) => {
		await ctx.step("start", async () => {
			const c = actorCtx<WorkflowHistorySimpleState>(ctx);
			c.state.status = "running";
			c.state.lastStep = "start";
			c.state.startedAt = Date.now();
			return { initialized: true };
		});

		await delay(700);

		await ctx.step("process", async () => {
			const c = actorCtx<WorkflowHistorySimpleState>(ctx);
			c.state.lastStep = "process";
			return { processed: true, items: 5 };
		});

		await delay(2200);

		await ctx.step("validate", async () => {
			const c = actorCtx<WorkflowHistorySimpleState>(ctx);
			c.state.lastStep = "validate";
			return { valid: true };
		});

		await delay(600);

		await ctx.step("complete", async () => {
			const c = actorCtx<WorkflowHistorySimpleState>(ctx);
			c.state.lastStep = "complete";
			c.state.status = "completed";
			c.state.completedAt = Date.now();
			c.state.output = { success: true, processedItems: 3 };
			return { success: true };
		});
	}),
});

const LOOP_ITEMS = ["A", "B", "C"];

export type WorkflowHistoryLoopState = {
	id: string;
	status: "running" | "completed";
	processed: number;
	batches: Array<{ index: number; item: string }>;
	completedAt?: number;
};

export const workflowHistoryLoop = actor({
	createState: (c): WorkflowHistoryLoopState => ({
		id: c.key[0] as string,
		status: "running",
		processed: 0,
		batches: [],
	}),
	actions: {
		getState: (c): WorkflowHistoryLoopState => c.state,
	},
	run: workflow(async (ctx) => {
		await ctx.step("init", async () => {
			const c = actorCtx<WorkflowHistoryLoopState>(ctx);
			c.state.status = "running";
			return { batchSize: LOOP_ITEMS.length };
		});

		await ctx.loop({
			name: "batch-loop",
			state: { index: 0 },
			commitInterval: 1,
			historyEvery: 1,
			historyKeep: LOOP_ITEMS.length,
			run: async (loopCtx, loopState: { index: number }) => {
				const item = LOOP_ITEMS[loopState.index];

				await loopCtx.step(`process-${loopState.index}`, async () => {
					const c = actorCtx<WorkflowHistoryLoopState>(loopCtx);
					c.state.processed += 1;
					c.state.batches.push({ index: loopState.index, item });
					return { item, status: "done" };
				});

				if (loopState.index >= LOOP_ITEMS.length - 1) {
					return Loop.break({ processed: LOOP_ITEMS.length });
				}

				return Loop.continue({ index: loopState.index + 1 });
			},
		});

		await ctx.step("finalize", async () => {
			const c = actorCtx<WorkflowHistoryLoopState>(ctx);
			c.state.status = "completed";
			c.state.completedAt = Date.now();
			return { allProcessed: true };
		});
	}),
});

export type WorkflowHistoryJoinState = {
	id: string;
	status: "pending" | "running" | "completed";
	result?: {
		api: string;
		rows: number;
		cacheHit: boolean;
	};
};

export const workflowHistoryJoin = actor({
	createState: (c): WorkflowHistoryJoinState => ({
		id: c.key[0] as string,
		status: "pending",
	}),
	actions: {
		getState: (c): WorkflowHistoryJoinState => c.state,
	},
	run: workflow(async (ctx) => {
		await ctx.step("start", async () => {
			const c = actorCtx<WorkflowHistoryJoinState>(ctx);
			c.state.status = "running";
			return { ready: true };
		});

		const results = await ctx.join("parallel-tasks", {
			"fetch-api": {
				run: async (branchCtx) => {
					await branchCtx.step("task-a", async () => {
						await delay(120);
						return { fetched: true };
					});
					return { data: "api-response" };
				},
			},
			"query-db": {
				run: async (branchCtx) => {
					await branchCtx.step("task-b", async () => {
						await delay(200);
						return { queried: true };
					});
					return { rows: 42 };
				},
			},
			"check-cache": {
				run: async (branchCtx) => {
					await branchCtx.step("task-c", async () => {
						await delay(60);
						return { checked: true };
					});
					return { hit: true };
				},
			},
		});

		await ctx.step("merge-results", async () => {
			const c = actorCtx<WorkflowHistoryJoinState>(ctx);
			c.state.status = "completed";
			c.state.result = {
				api: results["fetch-api"].data,
				rows: results["query-db"].rows,
				cacheHit: results["check-cache"].hit,
			};
			return { merged: true };
		});
	}),
});

export type WorkflowHistoryRaceState = {
	id: string;
	status: "running" | "completed";
	winner?: string;
	result?: string;
};

export const workflowHistoryRace = actor({
	createState: (c): WorkflowHistoryRaceState => ({
		id: c.key[0] as string,
		status: "running",
	}),
	actions: {
		getState: (c): WorkflowHistoryRaceState => c.state,
	},
	run: workflow(async (ctx) => {
		await ctx.step("begin", async () => {
			const c = actorCtx<WorkflowHistoryRaceState>(ctx);
			c.state.status = "running";
			return { started: true };
		});

		const { winner, value } = await ctx.race<{
			provider: string;
			latency: number;
		}>("race-providers", [
			{
				name: "provider-fast",
				run: async (branchCtx) => {
					await branchCtx.sleep("provider-fast-step", 50);
					return { provider: "cdn-edge", latency: 12 };
				},
			},
			{
				name: "provider-slow",
				run: async (branchCtx) => {
					await branchCtx.sleep("provider-slow-step", 200);
					return { provider: "origin", latency: 120 };
				},
			},
		]);

		await ctx.step("use-result", async () => {
			const c = actorCtx<WorkflowHistoryRaceState>(ctx);
			c.state.status = "completed";
			c.state.winner = winner;
			c.state.result = value.provider;
			return { used: value.provider };
		});
	}),
});

export type WorkflowHistoryFullState = {
	id: string;
	status: "pending" | "running" | "waiting" | "completed" | "failed";
	seededMessages: boolean;
	lastStep?: string;
	startedAt?: number;
	completedAt?: number;
};

const QUEUE_ORDER_CREATED = "order:created";
const QUEUE_ORDER_UPDATED = "order:updated";
const QUEUE_ORDER_ITEM = "order:item";
const QUEUE_ORDER_ARTIFACT = "order:artifact";
const QUEUE_ORDER_READY = "order:ready";
const QUEUE_ORDER_OPTIONAL = "order:optional";

type OrderCreatedMessage = { id: string };
type OrderUpdatedMessage = { id: string; status: string };
type OrderItemMessage = { sku: string; qty: number };
type OrderArtifactMessage = { artifactId: string };
type OrderReadyMessage = { batch: number };
type OrderOptionalMessage = { note?: string };

type MessageSeed =
	| { name: typeof QUEUE_ORDER_CREATED; payload: OrderCreatedMessage }
	| { name: typeof QUEUE_ORDER_UPDATED; payload: OrderUpdatedMessage }
	| { name: typeof QUEUE_ORDER_ITEM; payload: OrderItemMessage }
	| { name: typeof QUEUE_ORDER_ARTIFACT; payload: OrderArtifactMessage }
	| { name: typeof QUEUE_ORDER_READY; payload: OrderReadyMessage };

const FULL_WORKFLOW_MESSAGE_SEEDS: MessageSeed[] = [
	{ name: QUEUE_ORDER_CREATED, payload: { id: "order-1" } },
	{ name: QUEUE_ORDER_UPDATED, payload: { id: "order-1", status: "paid" } },
	{ name: QUEUE_ORDER_ITEM, payload: { sku: "sku-0", qty: 1 } },
	{ name: QUEUE_ORDER_ITEM, payload: { sku: "sku-4", qty: 1 } },
	{ name: QUEUE_ORDER_ARTIFACT, payload: { artifactId: "artifact-0" } },
	{ name: QUEUE_ORDER_ARTIFACT, payload: { artifactId: "artifact-1" } },
	{ name: QUEUE_ORDER_ARTIFACT, payload: { artifactId: "artifact-2" } },
	{ name: QUEUE_ORDER_READY, payload: { batch: 3 } },
	{ name: QUEUE_ORDER_READY, payload: { batch: 0 } },
	{ name: QUEUE_ORDER_READY, payload: { batch: 2 } },
];

const FULL_WORKFLOW_ITEMS = [
	{ id: "item-1", basePrice: 100, tax: 8 },
	{ id: "item-2", basePrice: 115, tax: 9 },
	{ id: "item-3", basePrice: 130, tax: 10 },
	{ id: "item-4", basePrice: 145, tax: 12 },
];

export const workflowHistoryFull = actor({
	createState: (c): WorkflowHistoryFullState => ({
		id: c.key[0] as string,
		status: "pending",
		seededMessages: false,
	}),
	queues: {
		[QUEUE_ORDER_CREATED]: queue<OrderCreatedMessage>(),
		[QUEUE_ORDER_UPDATED]: queue<OrderUpdatedMessage>(),
		[QUEUE_ORDER_ITEM]: queue<OrderItemMessage>(),
		[QUEUE_ORDER_ARTIFACT]: queue<OrderArtifactMessage>(),
		[QUEUE_ORDER_READY]: queue<OrderReadyMessage>(),
		[QUEUE_ORDER_OPTIONAL]: queue<OrderOptionalMessage>(),
	},
	actions: {
		getState: (c): WorkflowHistoryFullState => c.state,
		seedMessages: async (c) => {
			if (c.state.seededMessages) return;
			for (const seed of FULL_WORKFLOW_MESSAGE_SEEDS) {
				await c.queue.send(seed.name, seed.payload);
			}
			c.state.seededMessages = true;
		},
	},
	run: workflow(async (ctx) => {
		await ctx.step("bootstrap", async () => {
			const c = actorCtx<WorkflowHistoryFullState>(ctx);
			c.state.status = "running";
			c.state.lastStep = "bootstrap";
			c.state.startedAt = Date.now();
			return {
				requestId: `req-${c.state.id}`,
				startedAt: Date.now(),
			};
		});

		await ctx.step("validate-input", async () => {
			const c = actorCtx<WorkflowHistoryFullState>(ctx);
			c.state.lastStep = "validate-input";
			return true;
		});

		await ctx.rollbackCheckpoint("checkpoint-after-validation");

		await ctx.step("load-user-profile", async () => {
			const c = actorCtx<WorkflowHistoryFullState>(ctx);
			c.state.lastStep = "load-user-profile";
			return {
				id: "user-123",
				tier: "standard",
				flags: ["email-verified", "promo-eligible"],
			};
		});

		await ctx.step("compute-discount", async () => {
			const c = actorCtx<WorkflowHistoryFullState>(ctx);
			c.state.lastStep = "compute-discount";
			return { percent: 5, reason: "tier-discount" };
		});

		await ctx.step("ephemeral-cache-check", async () => {
			const c = actorCtx<WorkflowHistoryFullState>(ctx);
			c.state.lastStep = "ephemeral-cache-check";
			return { cacheHit: false, tier: "standard" };
		});

		await ctx.rollbackCheckpoint("checkpoint-before-reserve");

		await ctx.loop({
			name: "process-items-loop",
			state: { index: 0 },
			commitInterval: 1,
			historyEvery: 1,
			historyKeep: 2,
			run: async (loopCtx, loopState: { index: number }) => {
				const item = FULL_WORKFLOW_ITEMS[loopState.index];
				if (!item) {
					return Loop.break({ count: FULL_WORKFLOW_ITEMS.length });
				}

				await loopCtx.step(`fetch-item-${loopState.index}`, async () => {
					return { itemId: item.id, basePrice: item.basePrice };
				});

				await loopCtx.step(`compute-tax-${loopState.index}`, async () => {
					return item.tax;
				});

				await loopCtx.step(
					`reserve-inventory-${loopState.index}`,
					async () => ({
						reservationId: `res-${loopState.index}`,
						itemId: item.id,
					}),
				);

				if (loopState.index >= FULL_WORKFLOW_ITEMS.length - 1) {
					return Loop.break({
						count: FULL_WORKFLOW_ITEMS.length,
						total: 504,
					});
				}

				return Loop.continue({ index: loopState.index + 1 });
			},
		});

		await ctx.sleep("short-cooldown", 40);
		await ctx.sleep("cooldown-sleep", 60);
		await ctx.sleep("wait-until-deadline", 45);

		await ctx.step("compute-deadlines", async () => {
			const readyBy = Date.now() + 800;
			const readyBatchBy = Date.now() + 1100;
			return { readyBy, readyBatchBy };
		});

		await ctx.queue.next("listen-order-created", {
			names: [QUEUE_ORDER_CREATED],
		});
		await ctx.queue.nextBatch("listen-order-updated-timeout", {
			names: [QUEUE_ORDER_UPDATED],
			timeout: 250,
		});
		await ctx.queue.nextBatch("listen-batch-two", {
			names: [QUEUE_ORDER_ITEM],
			count: 2,
		});
		await ctx.queue.nextBatch("listen-artifacts-timeout", {
			names: [QUEUE_ORDER_ARTIFACT],
			count: 3,
			timeout: 300,
		});
		await ctx.queue.nextBatch("listen-optional", {
			names: [QUEUE_ORDER_OPTIONAL],
			timeout: 200,
		});
		await ctx.queue.nextBatch("listen-until", {
			names: [QUEUE_ORDER_READY],
			timeout: 300,
		});
		await ctx.queue.nextBatch("listen-batch-until", {
			names: [QUEUE_ORDER_READY],
			count: 2,
			timeout: 400,
		});

		await ctx.join("join-dependencies", {
			inventory: {
				run: async (branchCtx) => {
					const reserved = await branchCtx.step(
						"inventory-audit",
						async () => 4,
					);
					await branchCtx.sleep("join-inventory-sleep", 35);
					return {
						reserved,
						checked: 4,
						notes: ["inventory-ok", "items=4"],
					};
				},
			},
			pricing: {
				run: async (branchCtx) => {
					const method = await branchCtx.step(
						"pricing-method",
						async () => "promo",
					);
					return {
						subtotal: 504,
						discount: 25,
						total: 479,
						method,
					};
				},
			},
			shipping: {
				run: async (branchCtx) => {
					const zone = await branchCtx.step(
						"shipping-zone",
						async () => "us-east",
					);
					await branchCtx.sleep("join-shipping-sleep", 35);
					return { method: "ground", etaDays: 4, zone };
				},
			},
		});

		await ctx.race("race-fulfillment", [
			{
				name: "race-fast",
				run: async (branchCtx) => {
					await branchCtx.sleep("race-fast-sleep", 70);
					return { method: "express", cost: 18, etaDays: 1 };
				},
			},
			{
				name: "race-slow",
				run: async (branchCtx) => {
					await branchCtx.sleep("race-slow-sleep", 250);
					return { method: "ground", cost: 8, etaDays: 4 };
				},
			},
		]);

		await ctx.removed("legacy-step-placeholder", "step");

		await ctx.step("finalize", async () => {
			const c = actorCtx<WorkflowHistoryFullState>(ctx);
			c.state.lastStep = "finalize";
			c.state.status = "completed";
			c.state.completedAt = Date.now();
			return true;
		});
	}),
});

export type WorkflowHistoryInProgressState = {
	id: string;
	status: "running" | "completed";
	processingDurationMs: number;
	progress: number;
	startedAt?: number;
	completedAt?: number;
};

export type WorkflowHistoryInProgressInput = {
	processingDurationMs?: number;
};

export const workflowHistoryInProgress = actor({
	createState: (
		c,
		input?: WorkflowHistoryInProgressInput,
	): WorkflowHistoryInProgressState => ({
		id: c.key[0] as string,
		status: "running",
		processingDurationMs: input?.processingDurationMs ?? 30000,
		progress: 0,
	}),
	actions: {
		getState: (c): WorkflowHistoryInProgressState => c.state,
	},
	run: workflow(async (ctx) => {
		await ctx.step("init", async () => {
			const c = actorCtx<WorkflowHistoryInProgressState>(ctx);
			c.state.startedAt = Date.now();
			c.state.progress = 10;
			return { initialized: true };
		});

		await ctx.step("fetch-data", async () => {
			const c = actorCtx<WorkflowHistoryInProgressState>(ctx);
			c.state.progress = 25;
			return { fetched: true, records: 100 };
		});

		await ctx.step("process", async () => {
			const c = actorCtx<WorkflowHistoryInProgressState>(ctx);
			c.state.progress = 42;
			await delay(c.state.processingDurationMs);
			c.state.status = "completed";
			c.state.completedAt = Date.now();
			return { processed: true };
		});
	}),
});

export type WorkflowHistoryRetryingState = {
	id: string;
	status: "running" | "completed";
	attempts: number;
	lastError?: string;
	succeedAfter: number;
};

const RETRY_MAX_RETRIES = 20;

export const workflowHistoryRetrying = actor({
	createState: (c): WorkflowHistoryRetryingState => ({
		id: c.key[0] as string,
		status: "running",
		attempts: 0,
		succeedAfter: 999,
	}),
	actions: {
		getState: (c): WorkflowHistoryRetryingState => c.state,
		allowSuccess: (c, afterAttempt?: number) => {
			c.state.succeedAfter = afterAttempt ?? c.state.attempts + 1;
		},
	},
	run: workflow(async (ctx) => {
		await ctx.step("start", async () => {
			const c = actorCtx<WorkflowHistoryRetryingState>(ctx);
			c.state.status = "running";
			return { ready: true };
		});

		await ctx.step({
			name: "api-call",
			maxRetries: RETRY_MAX_RETRIES,
			retryBackoffBase: 250,
			retryBackoffMax: 1500,
			run: async () => {
				const c = actorCtx<WorkflowHistoryRetryingState>(ctx);
				c.state.attempts += 1;
				if (c.state.attempts < c.state.succeedAfter) {
					const error = new Error("Connection timeout after 5000ms");
					c.state.lastError = error.message;
					throw error;
				}
				c.state.status = "completed";
				c.state.lastError = undefined;
				return { success: true };
			},
		});
	}),
});

export type WorkflowHistoryFailedState = {
	id: string;
	status: "running" | "failed";
	attempts: number;
	lastError?: string;
};

const FAILED_MAX_RETRIES = 3;

export const workflowHistoryFailed = actor({
	createState: (c): WorkflowHistoryFailedState => ({
		id: c.key[0] as string,
		status: "running",
		attempts: 0,
	}),
	actions: {
		getState: (c): WorkflowHistoryFailedState => c.state,
	},
	run: workflow(async (ctx) => {
		await ctx.step("init", async () => {
			const c = actorCtx<WorkflowHistoryFailedState>(ctx);
			c.state.status = "running";
			return { initialized: true };
		});

		await ctx.step("validate", async () => {
			return { valid: true };
		});

		await ctx.step({
			name: "process",
			maxRetries: FAILED_MAX_RETRIES,
			retryBackoffBase: 200,
			retryBackoffMax: 800,
			run: async () => {
				const c = actorCtx<WorkflowHistoryFailedState>(ctx);
				c.state.attempts += 1;
				const error = new Error(
					"Database connection failed: ECONNREFUSED",
				);
				c.state.lastError = error.message;
				throw error;
			},
		});
	}),
});
