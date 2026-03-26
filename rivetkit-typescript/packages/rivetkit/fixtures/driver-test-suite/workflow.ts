// @ts-nocheck
import { Loop } from "@rivetkit/workflow-engine";
import { actor, event, queue } from "@/actor/mod";
import { db } from "@/db/mod";
import { WORKFLOW_GUARD_KV_KEY } from "@/workflow/constants";
import {
	type WorkflowErrorEvent,
	type WorkflowLoopContextOf,
	workflow,
} from "@/workflow/mod";
import type { registry } from "./registry";

const WORKFLOW_QUEUE_NAME = "workflow-default";
const WORKFLOW_NESTED_QUEUE_NAME = "workflow-nested";

export const workflowCounterActor = actor({
	state: {
		runCount: 0,
		guardTriggered: false,
		history: [] as number[],
	},
	run: workflow(async (ctx) => {
		await ctx.loop("counter", async (loopCtx) => {
			try {
				// Accessing state outside a step should throw.
				// biome-ignore lint/style/noUnusedExpressions: intentionally checking accessor.
				loopCtx.state;
			} catch {}

			await loopCtx.step("increment", async () => {
				incrementWorkflowCounter(loopCtx);
			});

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
			await loopCtx.step("store-message", async () => {
				await storeWorkflowQueueMessage(
					loopCtx,
					message.body,
					complete,
				);
			});
			return Loop.continue(undefined);
		});
	}),
	actions: {
		getMessages: (c) => c.state.received,
		sendAndWait: async (c, payload: unknown) => {
			const client = c.client<typeof registry>();
			const handle = client.workflowQueueActor.getForId(c.actorId);
			return await handle.send(WORKFLOW_QUEUE_NAME, payload, {
				wait: true,
				timeout: 1_000,
			});
		},
	},
});

export const workflowNestedLoopActor = actor({
	state: {
		processed: [] as string[],
	},
	queues: {
		[WORKFLOW_NESTED_QUEUE_NAME]: queue<
			{ items: string[] },
			{ processed: number }
		>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop("command-loop", async (loopCtx) => {
			const message = await loopCtx.queue.next<{
				items: string[];
			}>("wait", {
				names: [WORKFLOW_NESTED_QUEUE_NAME],
				completable: true,
			});
			let itemIndex = 0;
			await loopCtx.loop("process-items", async (subLoopCtx) => {
				const item = message.body.items[itemIndex];
				if (item === undefined) {
					return Loop.break(undefined);
				}

				await subLoopCtx.step(`process-item-${itemIndex}`, async () => {
					subLoopCtx.state.processed.push(item);
				});
				itemIndex += 1;
				return Loop.continue(undefined);
			});

			await message.complete?.({ processed: message.body.items.length });
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

export const workflowNestedJoinActor = actor({
	state: {
		processed: [] as string[],
	},
	queues: {
		[WORKFLOW_NESTED_QUEUE_NAME]: queue<
			{ items: string[] },
			{ processed: number }
		>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop("command-loop", async (loopCtx) => {
			const message = await loopCtx.queue.next<{
				items: string[];
			}>("wait", {
				names: [WORKFLOW_NESTED_QUEUE_NAME],
				completable: true,
			});

			await loopCtx.join(
				"process-items",
				Object.fromEntries(
					message.body.items.map((item, index) => [
						`item-${index}`,
						{
							run: async (branchCtx) =>
								await branchCtx.step(
									`process-item-${index}`,
									async () => {
										branchCtx.state.processed.push(item);
										return item;
									},
								),
						},
					]),
				),
			);

			await message.complete?.({ processed: message.body.items.length });
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

export const workflowNestedRaceActor = actor({
	state: {
		processed: [] as string[],
	},
	queues: {
		[WORKFLOW_NESTED_QUEUE_NAME]: queue<
			{ items: string[] },
			{ processed: number }
		>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop("command-loop", async (loopCtx) => {
			const message = await loopCtx.queue.next<{
				items: string[];
			}>("wait", {
				names: [WORKFLOW_NESTED_QUEUE_NAME],
				completable: true,
			});
			const item = message.body.items[0];

			if (item !== undefined) {
				await loopCtx.race("process-item", [
					{
						name: "fast",
						run: async (raceCtx) =>
							await raceCtx.step("process-fast", async () => {
								raceCtx.state.processed.push(item);
								return item;
							}),
					},
					{
						name: "slow",
						run: async (raceCtx) => {
							await new Promise<void>((resolve) => {
								if (raceCtx.abortSignal.aborted) {
									resolve();
									return;
								}
								raceCtx.abortSignal.addEventListener(
									"abort",
									() => resolve(),
									{ once: true },
								);
							});
							return "slow";
						},
					},
				]);
			}

			await message.complete?.({ processed: message.body.items.length });
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

export const workflowSpawnChildActor = actor({
	createState: (_c, input?: string) => ({
		label: input ?? "",
		started: false,
		processed: [] as string[],
	}),
	queues: {
		work: queue<{ task: string }, { ok: true }>(),
	},
	run: workflow(async (ctx) => {
		await ctx.step("mark-started", async () => {
			ctx.state.started = true;
		});

		await ctx.loop("cmd-loop", async (loopCtx) => {
			const message = await loopCtx.queue.next<{ task: string }>(
				"wait-cmd",
				{
					names: ["work"],
					completable: true,
				},
			);
			await loopCtx.step("process-cmd", async () => {
				loopCtx.state.processed.push(message.body.task);
			});
			await message.complete?.({ ok: true });
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

export const workflowSpawnParentActor = actor({
	state: {
		results: [] as Array<{
			key: string;
			result: unknown | null;
			error: string | null;
		}>,
	},
	queues: {
		spawn: queue<{ key: string }>(),
	},
	run: workflow(async (ctx) => {
		await ctx.loop("parent-loop", async (loopCtx) => {
			const message = await loopCtx.queue.next<{ key: string }>(
				"wait-parent",
				{
					names: ["spawn"],
					completable: true,
				},
			);

			await loopCtx.step("spawn-child", async () => {
				try {
					const client = loopCtx.client<typeof registry>();
					const handle = client.workflowSpawnChildActor.getOrCreate(
						[message.body.key],
						{
							createWithInput: message.body.key,
						},
					);
					const result = await handle.send(
						"work",
						{ task: "hello" },
						{
							wait: true,
							timeout: 500,
						},
					);
					loopCtx.state.results.push({
						key: message.body.key,
						result,
						error: null,
					});
				} catch (error) {
					loopCtx.state.results.push({
						key: message.body.key,
						result: null,
						error:
							error instanceof Error
								? error.message
								: String(error),
					});
				}
			});

			await message.complete?.({ ok: true });
			return Loop.continue(undefined);
		});
	}),
	actions: {
		triggerSpawn: async (c, key: string) => {
			await c.queue.send("spawn", { key });
			return { queued: true };
		},
		getState: (c) => c.state,
	},
	options: {
		sleepTimeout: 50,
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
		await ctx.loop("access", async (loopCtx) => {
			let outsideDbError: string | null = null;
			let outsideClientError: string | null = null;

			try {
				// Accessing db outside a step should throw.
				// biome-ignore lint/style/noUnusedExpressions: intentionally checking accessor.
				loopCtx.db;
			} catch (error) {
				outsideDbError =
					error instanceof Error ? error.message : String(error);
			}

			try {
				loopCtx.client<typeof registry>();
			} catch (error) {
				outsideClientError =
					error instanceof Error ? error.message : String(error);
			}

			await loopCtx.step("access-step", async () => {
				await updateWorkflowAccessState(
					loopCtx,
					outsideDbError,
					outsideClientError,
				);
			});

			await loopCtx.sleep("idle", 25);
			return Loop.continue(undefined);
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
		await ctx.loop("sleep", async (loopCtx) => {
			await loopCtx.step("tick", async () => {
				incrementWorkflowSleepTick(loopCtx);
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
		await ctx.loop("wait-forever", async (loopCtx) => {
			await loopCtx.queue.next("wait-for-never", {
				names: ["never"],
			});
			return Loop.continue(undefined);
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

export const workflowCompleteActor = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		runCount: 0,
	},
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	run: workflow(async (ctx) => {
		await ctx.step("complete", async () => {
			ctx.state.runCount += 1;
		});
	}),
	actions: {
		getState: (c) => c.state,
	},
	options: {
		sleepTimeout: 50,
	},
});

export const workflowDestroyActor = actor({
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		const observer = client.destroyObserver.getOrCreate(["observer"]);
		await observer.notifyDestroyed(c.key.join("/"));
	},
	run: workflow(async (ctx) => {
		await ctx.step("destroy", async () => {
			ctx.destroy();
		});
	}),
});

export const workflowFailedStepActor = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		timeline: [] as string[],
		runCount: 0,
	},
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	run: workflow(async (ctx) => {
		await ctx.step("prepare", async () => {
			ctx.state.timeline.push("prepare");
		});
		await ctx.step({
			name: "fail",
			maxRetries: 2,
			run: async () => {
				ctx.state.runCount += 1;
				ctx.state.timeline.push("fail");
				throw new Error("workflow step failed");
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

export const workflowErrorHookActor = actor({
	state: {
		attempts: 0,
		events: [] as WorkflowErrorEvent[],
	},
	run: workflow(
		async (ctx) => {
			await ctx.step({
				name: "flaky",
				maxRetries: 2,
				retryBackoffBase: 1,
				retryBackoffMax: 1,
				run: async () => {
					ctx.state.attempts += 1;
					if (ctx.state.attempts === 1) {
						throw new Error("workflow hook failed");
					}
				},
			});
			await ctx.sleep("idle", 60_000);
		},
		{
			onError: (c, event) => {
				c.state.events.push(event);
			},
		},
	),
	actions: {
		getErrorState: (c) => c.state,
	},
});

export const workflowErrorHookSleepActor = actor({
	state: {
		attempts: 0,
		wakeCount: 0,
		sleepCount: 0,
		events: [] as WorkflowErrorEvent[],
	},
	onWake: (c) => {
		c.state.wakeCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	run: workflow(
		async (ctx) => {
			await ctx.step({
				name: "flaky",
				maxRetries: 2,
				retryBackoffBase: 1,
				retryBackoffMax: 1,
				run: async () => {
					ctx.state.attempts += 1;
					if (ctx.state.attempts === 1) {
						throw new Error("workflow hook failed");
					}
				},
			});
			await ctx.sleep("idle", 60_000);
		},
		{
			onError: (c, event) => {
				c.state.events.push(event);
			},
		},
	),
	actions: {
		getErrorState: (c) => c.state,
		triggerSleep: (c) => {
			c.sleep();
		},
	},
});

export const workflowErrorHookEffectsActor = actor({
	state: {
		attempts: 0,
		lastError: null as WorkflowErrorEvent | null,
		errorCount: 0,
	},
	events: {
		workflowError: event<[WorkflowErrorEvent]>(),
	},
	queues: {
		start: queue<null>(),
		errors: queue<WorkflowErrorEvent>(),
	},
	run: workflow(
		async (ctx) => {
			await ctx.queue.next("start", {
				names: ["start"],
			});
			await ctx.step({
				name: "flaky",
				maxRetries: 2,
				retryBackoffBase: 1,
				retryBackoffMax: 1,
				run: async () => {
					ctx.state.attempts += 1;
					if (ctx.state.attempts === 1) {
						throw new Error("workflow hook failed");
					}
				},
			});
			await ctx.sleep("idle", 60_000);
		},
		{
			onError: async (c, event) => {
				c.state.lastError = event;
				c.state.errorCount += 1;
				c.broadcast("workflowError", event);
				await c.queue.send("errors", event);
			},
		},
	),
	actions: {
		getErrorState: (c) => c.state,
		startWorkflow: async (c) => {
			const client = c.client<typeof registry>();
			const handle = client.workflowErrorHookEffectsActor.getForId(
				c.actorId,
			);
			await handle.send("start", null);
		},
		receiveQueuedError: async (c) => {
			const message = await c.queue.next({
				names: ["errors"],
				timeout: 1_000,
			});
			return message?.body ?? null;
		},
	},
});

export const workflowReplayActor = actor({
	state: {
		timeline: [] as string[],
	},
	run: workflow(async (ctx) => {
		await ctx.step("one", async () => {
			ctx.state.timeline.push("one");
		});
		await ctx.step("two", async () => {
			ctx.state.timeline.push("two");
		});
	}),
	actions: {
		getTimeline: (c) => [...c.state.timeline],
	},
	options: {
		sleepTimeout: 50,
	},
});

export const workflowRunningStepActor = actor({
	state: {
		preparedAt: null as number | null,
		startedAt: null as number | null,
	},
	run: workflow(async (ctx) => {
		await ctx.step("prepare", async () => {
			ctx.state.preparedAt = Date.now();
		});
		await ctx.step({
			name: "block",
			timeout: 0,
			run: async () => {
				ctx.state.startedAt = Date.now();
				await new Promise((resolve) => setTimeout(resolve, 250));
			},
		});
	}),
	actions: {
		getState: (c) => ({ ...c.state }),
	},
	options: {
		sleepTimeout: 50,
	},
});

function incrementWorkflowCounter(
	ctx: WorkflowLoopContextOf<typeof workflowCounterActor>,
): void {
	ctx.state.runCount += 1;
	ctx.state.history.push(ctx.state.runCount);
}

async function storeWorkflowQueueMessage(
	ctx: WorkflowLoopContextOf<typeof workflowQueueActor>,
	body: unknown,
	complete: (response: { echo: unknown }) => Promise<void>,
): Promise<void> {
	ctx.state.received.push(body);
	await complete({ echo: body });
}

async function updateWorkflowAccessState(
	ctx: WorkflowLoopContextOf<typeof workflowAccessActor>,
	outsideDbError: string | null,
	outsideClientError: string | null,
): Promise<void> {
	await ctx.db.execute(
		`INSERT INTO workflow_access_log (created_at) VALUES (${Date.now()})`,
	);
	const counts = await ctx.db.execute<{ count: number }>(
		`SELECT COUNT(*) as count FROM workflow_access_log`,
	);
	const client = ctx.client<typeof registry>();

	ctx.state.outsideDbError = outsideDbError;
	ctx.state.outsideClientError = outsideClientError;
	ctx.state.insideDbCount = counts[0]?.count ?? 0;
	ctx.state.insideClientAvailable =
		typeof client.workflowQueueActor.getForId === "function";
}

function incrementWorkflowSleepTick(
	ctx: WorkflowLoopContextOf<typeof workflowSleepActor>,
): void {
	ctx.state.ticks += 1;
}

export { WORKFLOW_NESTED_QUEUE_NAME, WORKFLOW_QUEUE_NAME };
