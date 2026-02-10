import { actor } from "rivetkit";
import { Loop, workflow, workflowQueueName } from "rivetkit/workflow";

const WORKFLOW_GUARD_KV_KEY = "__rivet_actor_workflow_guard_triggered";

const WORKFLOW_QUEUE_NAME = "workflow-default";

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
	run: workflow(async (ctx) => {
		await ctx.loop({
			name: "queue",
			run: async (loopCtx) => {
				const actorLoopCtx = loopCtx as any;
				const message = await loopCtx.listen(
					"queue-wait",
					WORKFLOW_QUEUE_NAME,
				);
				await loopCtx.step("store-message", async () => {
					actorLoopCtx.state.received.push(message.body);
					await message.complete({ echo: message.body });
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

export { WORKFLOW_QUEUE_NAME, workflowQueueName };
