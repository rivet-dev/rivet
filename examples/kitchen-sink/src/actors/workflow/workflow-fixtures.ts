import { actor, event, queue } from "rivetkit";
import { Loop, type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

const WORKFLOW_GUARD_KV_KEY = "__rivet_actor_workflow_guard_triggered";

const WORKFLOW_QUEUE_NAME = "workflow-default";
const WORKFLOW_TIMEOUT_QUEUE_NAME = "workflow-timeout";

export const workflowCounterActor = actor({
	state: {
		runCount: 0,
		guardTriggered: false,
		history: [] as number[],
	},
	run: workflow(async (ctx) => {
		let leakedStep:
			| WorkflowStepContextOf<typeof workflowCounterActor>
			| undefined;
		await ctx.loop("counter", async (loopCtx) => {
			await loopCtx.step("increment", async (step) => {
				incrementWorkflowCounter(step);
				// Capture the step context to verify it cannot be used after
				// its step has finished.
				leakedStep = step;
			});

			if (leakedStep) {
				try {
					// Accessing state on a finished step context should throw.
					// biome-ignore lint/style/noUnusedExpressions: intentionally checking accessor.
					leakedStep.state;
				} catch {}
			}

			await loopCtx.sleep("idle", 25);
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getState: async (c) => {
			const guardFlag = await c.kv.get(WORKFLOW_GUARD_KV_KEY);
			if (guardFlag === "true") {
				c.state.guardTriggered = true;
			}
			return c.state;
		},
	},
	options: {
		sleepTimeout: 50,
	},
});

export const workflowQueueActor = actor({
	state: {
		received: [] as unknown[],
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
			await loopCtx.step("store-message", async (step) => {
				await storeWorkflowQueueMessage(step, message.body, complete);
			});
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getMessages: (c) => c.state.received,
	},
});

export const workflowSleepActor = actor({
	state: {
		ticks: 0,
	},
	run: workflow(async (ctx) => {
		await ctx.loop("sleep", async (loopCtx) => {
			await loopCtx.step("tick", async (step) => {
				incrementWorkflowSleepTick(step);
			});
			await loopCtx.sleep("delay", 40);
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getState: (c) => c.state,
	},
	options: {
		sleepTimeout: 50,
	},
});

export const workflowQueueTimeoutActor = actor({
	state: {
		processed: 0,
		ticks: 0,
		lastTickAt: null as number | null,
		lastJob: null as { id: string; payload: string } | null,
		timeoutMs: 2_000,
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
			const timeoutMs = await loopCtx.step("read-timeout", async (step) =>
				readWorkflowTimeoutMs(step),
			);

			const [message] = await loopCtx.queue.nextBatch(
				"wait-job-or-timeout",
				{
					names: [WORKFLOW_TIMEOUT_QUEUE_NAME],
					timeout: timeoutMs,
				},
			);

			if (!message) {
				await loopCtx.step("tick", async (step) => {
					recordWorkflowTimeoutTick(step);
				});
				return Loop.continue(undefined);
			}

			await loopCtx.step("process-job", async (step) => {
				processWorkflowTimeoutJob(step, message.body);
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
		setTimeoutMs: (c, timeoutMs: number) => {
			c.state.timeoutMs = Math.max(100, Math.floor(timeoutMs));
			return c.state.timeoutMs;
		},
		getState: (c) => c.state,
	},
});

function incrementWorkflowCounter(
	step: WorkflowStepContextOf<typeof workflowCounterActor>,
): void {
	step.state.runCount += 1;
	step.state.history.push(step.state.runCount);
}

async function storeWorkflowQueueMessage(
	step: WorkflowStepContextOf<typeof workflowQueueActor>,
	body: unknown,
	complete: (response: { echo: unknown }) => Promise<void>,
): Promise<void> {
	step.state.received.push(body);
	await complete({ echo: body });
}

function incrementWorkflowSleepTick(
	step: WorkflowStepContextOf<typeof workflowSleepActor>,
): void {
	step.state.ticks += 1;
}

function readWorkflowTimeoutMs(
	step: WorkflowStepContextOf<typeof workflowQueueTimeoutActor>,
): number {
	return step.state.timeoutMs;
}

function recordWorkflowTimeoutTick(
	step: WorkflowStepContextOf<typeof workflowQueueTimeoutActor>,
): void {
	const at = Date.now();
	step.state.ticks += 1;
	step.state.lastTickAt = at;
	step.broadcast("tick", {
		ticks: step.state.ticks,
		at,
	});
}

function processWorkflowTimeoutJob(
	step: WorkflowStepContextOf<typeof workflowQueueTimeoutActor>,
	job: { id: string; payload: string },
): void {
	step.state.processed += 1;
	step.state.lastJob = job;
	step.broadcast("jobProcessed", {
		processed: step.state.processed,
		job,
	});
}

export { WORKFLOW_QUEUE_NAME, WORKFLOW_TIMEOUT_QUEUE_NAME };
