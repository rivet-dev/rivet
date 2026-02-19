import { Loop } from "@rivetkit/workflow-engine";
import { actor, queue } from "@/actor/mod";
import { db } from "@/db/mod";
import { WORKFLOW_GUARD_KV_KEY } from "@/workflow/constants";
import { workflow } from "@/workflow/mod";
import type { registry } from "./registry";

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
		sendAndWait: async (c, payload: unknown) => {
			const client = c.client<typeof registry>();
			const handle = client.workflowQueueActor.getForId(c.actorId);
			return await handle.send(
				WORKFLOW_QUEUE_NAME,
				payload,
				{ wait: true, timeout: 1_000 },
			);
		},
	},
});

export const workflowAccessActor = actor({
	db: db({
		onMigrate: async (rawDb) => {
			await rawDb.execute(`
				CREATE TABLE IF NOT EXISTS workflow_access_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	state: {
		outsideDbError: null as string | null,
		outsideClientError: null as string | null,
		insideDbCount: 0,
		insideClientAvailable: false,
	},
	run: workflow(async (ctx) => {
		await ctx.loop({
			name: "access",
			run: async (loopCtx) => {
				const actorLoopCtx = loopCtx as any;
				let outsideDbError: string | null = null;
				let outsideClientError: string | null = null;

				try {
					// Accessing db outside a step should throw.
					// biome-ignore lint/style/noUnusedExpressions: intentionally checking accessor.
					actorLoopCtx.db;
				} catch (error) {
					outsideDbError =
						error instanceof Error ? error.message : String(error);
				}

				try {
					actorLoopCtx.client();
				} catch (error) {
					outsideClientError =
						error instanceof Error ? error.message : String(error);
				}

				await loopCtx.step("access-step", async () => {
					await loopCtx.db.execute(
						`INSERT INTO workflow_access_log (created_at) VALUES (${Date.now()})`,
					);
					const counts = await loopCtx.db.execute<{ count: number }>(
						`SELECT COUNT(*) as count FROM workflow_access_log`,
					);
					const client = loopCtx.client<typeof registry>();

					loopCtx.state.outsideDbError = outsideDbError;
					loopCtx.state.outsideClientError = outsideClientError;
					loopCtx.state.insideDbCount = counts[0]?.count ?? 0;
					loopCtx.state.insideClientAvailable =
						typeof client.workflowQueueActor.getForId === "function";
				});

				await loopCtx.sleep("idle", 25);
				return Loop.continue(undefined);
			},
		});
	}),
	actions: {
		getState: (c) => c.state,
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

export const workflowStopTeardownActor = actor({
	state: {
		wakeAts: [] as number[],
		sleepAts: [] as number[],
	},
	queues: {
		never: queue<unknown>(),
	},
	onWake: (c) => {
		c.state.wakeAts.push(Date.now());
	},
	onSleep: (c) => {
		c.state.sleepAts.push(Date.now());
	},
	run: workflow(async (ctx) => {
		await ctx.loop({
			name: "wait-forever",
			run: async (loopCtx) => {
				await loopCtx.queue.next("wait-for-never", {
					names: ["never"],
				});
				return Loop.continue(undefined);
			},
		});
	}),
	actions: {
		getTimeline: (c) => ({
			wakeAts: [...c.state.wakeAts],
			sleepAts: [...c.state.sleepAts],
		}),
	},
	options: {
		sleepTimeout: 75,
		runStopTimeout: 2_000,
	},
});

export { WORKFLOW_QUEUE_NAME };
