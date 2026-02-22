import { actor, event, queue } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";

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
		await ctx.loop({
			name: "counter",
			run: async (loopCtx) => {
				const actorLoopCtx = loopCtx as any;
				try {
					// Accessing state outside a step should throw.
					// biome-ignore lint/style/noUnusedExpressions: intentionally checking accessor.
					actorLoopCtx.state;
				} catch {}

				await loopCtx.step("increment", async () => {
					actorLoopCtx.state.runCount += 1;
					actorLoopCtx.state.history.push(actorLoopCtx.state.runCount);
				});

				await loopCtx.sleep("idle", 25);
				return Loop.continue(undefined);
			},
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
		await ctx.loop({
			name: "queue",
			run: async (loopCtx) => {
				const actorLoopCtx = loopCtx as any;
				const [message] = await loopCtx.queue.next("queue-wait", {
					names: [WORKFLOW_QUEUE_NAME],
					completable: true,
				});
				if (!message || !message.complete) {
					return Loop.continue(undefined);
				}
				const complete = message.complete;
				await loopCtx.step("store-message", async () => {
					actorLoopCtx.state.received.push(message.body);
					await complete({ echo: message.body });
				});
				return Loop.continue(undefined);
			},
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
		await ctx.loop({
			name: "sleep",
			run: async (loopCtx) => {
				const actorLoopCtx = loopCtx as any;
				await loopCtx.step("tick", async () => {
					actorLoopCtx.state.ticks += 1;
				});
				await loopCtx.sleep("delay", 40);
				return Loop.continue(undefined);
			},
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
		jobProcessed: event<{ processed: number; job: { id: string; payload: string } }>(),
	},
	queues: {
		[WORKFLOW_TIMEOUT_QUEUE_NAME]: queue<{ id: string; payload: string }>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop({
			name: "queue-timeout-loop",
			run: async (loopCtx) => {
				const actorLoopCtx = loopCtx as any;
				const timeoutMs = await loopCtx.step("read-timeout", async () => {
					return actorLoopCtx.state.timeoutMs;
				});

				const [message] = await loopCtx.queue.next("wait-job-or-timeout", {
					names: [WORKFLOW_TIMEOUT_QUEUE_NAME],
					timeout: timeoutMs,
				});

				if (!message) {
					await loopCtx.step("tick", async () => {
						const at = Date.now();
						actorLoopCtx.state.ticks += 1;
						actorLoopCtx.state.lastTickAt = at;
						actorLoopCtx.broadcast("tick", {
							ticks: actorLoopCtx.state.ticks,
							at,
						});
					});
					return Loop.continue(undefined);
				}

				await loopCtx.step("process-job", async () => {
					actorLoopCtx.state.processed += 1;
					actorLoopCtx.state.lastJob = message.body;
					actorLoopCtx.broadcast("jobProcessed", {
						processed: actorLoopCtx.state.processed,
						job: message.body,
					});
				});
				return Loop.continue(undefined);
			},
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

export { WORKFLOW_QUEUE_NAME, WORKFLOW_TIMEOUT_QUEUE_NAME };
