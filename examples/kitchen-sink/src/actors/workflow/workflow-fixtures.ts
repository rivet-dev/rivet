import { actor, event, queue } from "rivetkit";
import { Loop, type WorkflowStepContextOf, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

const WORKFLOW_GUARD_KV_KEY = "__rivet_actor_workflow_guard_triggered";

const WORKFLOW_QUEUE_NAME = "workflow-default";
const WORKFLOW_TIMEOUT_QUEUE_NAME = "workflow-timeout";

type WorkflowCounterRecord = {
	runCount: number;
	guardTriggered: boolean;
	history: number[];
};

type WorkflowQueueRecord = {
	received: unknown[];
};

type WorkflowSleepRecord = {
	ticks: number;
};

type WorkflowQueueTimeoutRecord = {
	processed: number;
	ticks: number;
	lastTickAt: number | null;
	lastJob: { id: string; payload: string } | null;
	timeoutMs: number;
};

export const workflowCounterActor = actor({
	db: recordDb(),
	onCreate: async (c) => {
		await saveRecord<WorkflowCounterRecord>(c.db, {
			runCount: 0,
			guardTriggered: false,
			history: [],
		});
	},
	run: workflow(async (ctx) => {
		let leakedCtx:
			| WorkflowStepContextOf<typeof workflowCounterActor>
			| undefined;
		await ctx.loop("counter", async (loopCtx) => {
			await loopCtx.step("increment", async (c) => {
				await incrementWorkflowCounter(c);
				// Capture the step context to verify it cannot be used after
				// its step has finished.
				leakedCtx = c;
			});

			if (leakedCtx) {
				try {
					// Accessing the database on a finished step context should throw.
					// biome-ignore lint/style/noUnusedExpressions: intentionally checking accessor.
					leakedCtx.db;
				} catch {}
			}

			await loopCtx.sleep("idle", 25);
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getState: async (c) => {
			const guardFlag = await c.kv.get(WORKFLOW_GUARD_KV_KEY);
			const record = await loadRecord<WorkflowCounterRecord>(c.db);
			if (guardFlag === "true" && !record.guardTriggered) {
				return await updateRecord<WorkflowCounterRecord>(
					c.db,
					(record) => {
						record.guardTriggered = true;
					},
				);
			}
			return record;
		},
	},
	options: {
		sleepTimeout: 50,
	},
});

export const workflowQueueActor = actor({
	db: recordDb(),
	onCreate: async (c) => {
		await saveRecord<WorkflowQueueRecord>(c.db, { received: [] });
	},
	queues: {
		[WORKFLOW_QUEUE_NAME]: queue<unknown, { echo: unknown }>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop("queue", async (loopCtx) => {
			const message = await loopCtx.queue.next("queue-wait", {
				names: [WORKFLOW_QUEUE_NAME],
				completable: true,
			});
			if (!message.complete) {
				return Loop.continue(undefined);
			}
			const complete = message.complete;
			await loopCtx.step("store-message", async (c) => {
				await storeWorkflowQueueMessage(c, message.body, complete);
			});
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getMessages: async (c) => {
			const record = await loadRecord<WorkflowQueueRecord>(c.db);
			return record.received;
		},
	},
});

export const workflowSleepActor = actor({
	db: recordDb(),
	onCreate: async (c) => {
		await saveRecord<WorkflowSleepRecord>(c.db, { ticks: 0 });
	},
	run: workflow(async (ctx) => {
		await ctx.loop("sleep", async (loopCtx) => {
			await loopCtx.step("tick", async (c) => {
				await incrementWorkflowSleepTick(c);
			});
			await loopCtx.sleep("delay", 40);
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getState: (c) => loadRecord<WorkflowSleepRecord>(c.db),
	},
	options: {
		sleepTimeout: 50,
	},
});

export const workflowQueueTimeoutActor = actor({
	db: recordDb(),
	onCreate: async (c) => {
		await saveRecord<WorkflowQueueTimeoutRecord>(c.db, {
			processed: 0,
			ticks: 0,
			lastTickAt: null,
			lastJob: null,
			timeoutMs: 2_000,
		});
	},
	events: {
		tick: event<{ ticks: number; at: number }>(),
		jobProcessed: event<{
			processed: number;
			job: { id: string; payload: string };
		}>(),
	},
	queues: {
		[WORKFLOW_TIMEOUT_QUEUE_NAME]: queue<{ id: string; payload: string }>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop("queue-timeout-loop", async (loopCtx) => {
			const timeoutMs = await loopCtx.step("read-timeout", async (c) => {
				const record = await loadRecord<WorkflowQueueTimeoutRecord>(
					c.db,
				);
				return record.timeoutMs;
			});

			const [message] = await loopCtx.queue.nextBatch(
				"wait-job-or-timeout",
				{
					names: [WORKFLOW_TIMEOUT_QUEUE_NAME],
					timeout: timeoutMs,
				},
			);

			if (!message) {
				await loopCtx.step("tick", async (c) => {
					await recordWorkflowTimeoutTick(c);
				});
				return Loop.continue(undefined);
			}

			await loopCtx.step("process-job", async (c) => {
				await processWorkflowTimeoutJob(c, message.body);
			});
			return Loop.continue(undefined);
		});
	}),
	actions: {
		enqueueJob: async (c, payload: string) => {
			const job = { id: crypto.randomUUID(), payload };
			await c.queue.send(WORKFLOW_TIMEOUT_QUEUE_NAME, job);
			return job;
		},
		setTimeoutMs: async (c, timeoutMs: number) => {
			const updated = await updateRecord<WorkflowQueueTimeoutRecord>(
				c.db,
				(record) => {
					record.timeoutMs = Math.max(100, Math.floor(timeoutMs));
				},
			);
			return updated.timeoutMs;
		},
		getState: (c) => loadRecord<WorkflowQueueTimeoutRecord>(c.db),
	},
});

async function incrementWorkflowCounter(
	ctx: WorkflowStepContextOf<typeof workflowCounterActor>,
): Promise<void> {
	await updateRecord<WorkflowCounterRecord>(ctx.db, (record) => {
		record.runCount += 1;
		record.history.push(record.runCount);
	});
}

async function storeWorkflowQueueMessage(
	ctx: WorkflowStepContextOf<typeof workflowQueueActor>,
	body: unknown,
	complete: (response: { echo: unknown }) => Promise<void>,
): Promise<void> {
	await updateRecord<WorkflowQueueRecord>(ctx.db, (record) => {
		record.received.push(body);
	});
	await complete({ echo: body });
}

async function incrementWorkflowSleepTick(
	ctx: WorkflowStepContextOf<typeof workflowSleepActor>,
): Promise<void> {
	await updateRecord<WorkflowSleepRecord>(ctx.db, (record) => {
		record.ticks += 1;
	});
}

async function recordWorkflowTimeoutTick(
	ctx: WorkflowStepContextOf<typeof workflowQueueTimeoutActor>,
): Promise<void> {
	const at = Date.now();
	const updated = await updateRecord<WorkflowQueueTimeoutRecord>(
		ctx.db,
		(record) => {
			record.ticks += 1;
			record.lastTickAt = at;
		},
	);
	ctx.broadcast("tick", {
		ticks: updated.ticks,
		at,
	});
}

async function processWorkflowTimeoutJob(
	ctx: WorkflowStepContextOf<typeof workflowQueueTimeoutActor>,
	job: { id: string; payload: string },
): Promise<void> {
	const updated = await updateRecord<WorkflowQueueTimeoutRecord>(
		ctx.db,
		(record) => {
			record.processed += 1;
			record.lastJob = job;
		},
	);
	ctx.broadcast("jobProcessed", {
		processed: updated.processed,
		job,
	});
}

export { WORKFLOW_QUEUE_NAME, WORKFLOW_TIMEOUT_QUEUE_NAME };
