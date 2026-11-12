// BATCH PROCESSOR (Loops Demo)
// Demonstrates: Loop with persistent state (cursor) for batch processing
// One actor per batch job - actor key is the job ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

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

function fetchBatch(
	cursor: number,
	batchSize: number,
	totalItems: number,
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
	db: recordDb(),
	onCreate: async (c, input?: BatchJobInput) => {
		await saveRecord<BatchJob>(c.db, {
			id: c.key[0] as string,
			totalItems: input?.totalItems ?? 50,
			batchSize: input?.batchSize ?? 5,
			status: "running",
			processedTotal: 0,
			currentBatch: 0,
			batches: [],
			startedAt: Date.now(),
		});
	},
	events: {
		batchProcessed: event<BatchInfo>(),
		stateChanged: event<BatchJob>(),
		processingComplete: event<{
			totalBatches: number;
			totalItems: number;
		}>(),
	},

	actions: {
		getJob: (c): Promise<BatchJob> => loadRecord<BatchJob>(c.db),
	},

	run: workflow(async (ctx) => {
		await ctx.loop({
			name: "batch-loop",
			state: { cursor: 0 },
			run: async (batchCtx, loopState: { cursor: number }) => {
				const batch = await batchCtx.step("fetch-batch", async (c) => {
					const job = await loadRecord<BatchJob>(c.db);
					c.log.info({
						msg: "processing batch",
						jobId: job.id,
						cursor: loopState.cursor,
					});
					await new Promise((r) =>
						setTimeout(r, 200 + Math.random() * 300),
					);
					return fetchBatch(
						loopState.cursor,
						job.batchSize,
						job.totalItems,
					);
				});

				await batchCtx.step("process-batch", async (c) => {
					await new Promise((r) =>
						setTimeout(r, 300 + Math.random() * 500),
					);

					const batchInfo: BatchInfo = {
						id: loopState.cursor,
						count: batch.items.length,
						processedAt: Date.now(),
					};

					const updated = await updateRecord<BatchJob>(
						c.db,
						(job) => {
							job.currentBatch = loopState.cursor;
							job.processedTotal += batch.items.length;
							job.batches.push(batchInfo);
						},
					);
					c.broadcast("batchProcessed", batchInfo);
					c.broadcast("stateChanged", updated);

					c.log.info({
						msg: "batch processed",
						jobId: updated.id,
						cursor: loopState.cursor,
						count: batch.items.length,
					});
				});

				if (!batch.hasMore) {
					await batchCtx.step("mark-complete", async (c) => {
						const updated = await updateRecord<BatchJob>(
							c.db,
							(job) => {
								job.status = "completed";
								job.completedAt = Date.now();
							},
						);
						c.broadcast("stateChanged", updated);
						c.broadcast("processingComplete", {
							totalBatches: loopState.cursor + 1,
							totalItems: updated.processedTotal,
						});
					});
					return Loop.break(loopState.cursor + 1);
				}

				return Loop.continue({ cursor: loopState.cursor + 1 });
			},
		});
	}),
});
