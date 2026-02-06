// BATCH PROCESSOR (Loops Demo)
// Demonstrates: Loop with persistent state (cursor) for batch processing
// One actor per batch job - actor key is the job ID

import { actor } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

export type BatchInfo = {
	id: number;
	count: number;
	processedAt: number;
};

export type BatchJob = {
	id: string;
	totalItems: number;
	batchSize: number;
	status: "running" | "stopped" | "completed";
	processedTotal: number;
	currentBatch: number;
	batches: BatchInfo[];
	startedAt: number;
	completedAt?: number;
};

type State = BatchJob;

function fetchBatch(
	cursor: number,
	batchSize: number,
	totalItems: number
): { items: number[]; hasMore: boolean } {
	const start = cursor * batchSize;
	const end = Math.min(start + batchSize, totalItems);
	const items = [];
	for (let i = start; i < end; i++) {
		items.push(i);
	}
	return {
		items,
		hasMore: end < totalItems,
	};
}

export type BatchJobInput = {
	totalItems?: number;
	batchSize?: number;
};

export const batch = actor({
	createState: (c, input?: BatchJobInput): BatchJob => ({
		id: c.key[0] as string,
		totalItems: input?.totalItems ?? 50,
		batchSize: input?.batchSize ?? 5,
		status: "running",
		processedTotal: 0,
		currentBatch: 0,
		batches: [],
		startedAt: Date.now(),
	}),

	actions: {
		getJob: (c): BatchJob => c.state,
	},

	run: workflow(async (ctx) => {
		await ctx.loop({
			name: "batch-loop",
			state: { cursor: 0 },
			run: async (batchCtx, loopState: { cursor: number }) => {
				const c = actorCtx<State>(batchCtx);

				const batch = await batchCtx.step("fetch-batch", async () => {
					ctx.log.info({
						msg: "processing batch",
						jobId: c.state.id,
						cursor: loopState.cursor,
					});
					await new Promise((r) => setTimeout(r, 200 + Math.random() * 300));
					return fetchBatch(loopState.cursor, c.state.batchSize, c.state.totalItems);
				});

				await batchCtx.step("process-batch", async () => {
					await new Promise((r) => setTimeout(r, 300 + Math.random() * 500));

					const batchInfo: BatchInfo = {
						id: loopState.cursor,
						count: batch.items.length,
						processedAt: Date.now(),
					};

					c.state.currentBatch = loopState.cursor;
					c.state.processedTotal += batch.items.length;
					c.state.batches.push(batchInfo);

					c.broadcast("batchProcessed", batchInfo);
					c.broadcast("stateChanged", c.state);

					ctx.log.info({
						msg: "batch processed",
						jobId: c.state.id,
						cursor: loopState.cursor,
						count: batch.items.length,
					});
				});

				if (!batch.hasMore) {
					await batchCtx.step("mark-complete", async () => {
						c.state.status = "completed";
						c.state.completedAt = Date.now();
						c.broadcast("stateChanged", c.state);
						c.broadcast("processingComplete", {
							totalBatches: loopState.cursor + 1,
							totalItems: c.state.processedTotal,
						});
					});
					return Loop.break(loopState.cursor + 1);
				}

				return Loop.continue({ cursor: loopState.cursor + 1 });
			},
		});
	}),
});
