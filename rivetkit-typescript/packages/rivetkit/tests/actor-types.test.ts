import { describe, expectTypeOf, it } from "vitest";
import { actor, event, queue } from "@/actor/mod";
import type { ActorContext, ActorContextOf } from "@/actor/contexts";
import type { ActorDefinition } from "@/actor/definition";
import type { DatabaseProviderContext } from "@/db/config";
import { db } from "@/db/mod";
import type { WorkflowContextOf as WorkflowContextOfFromRoot } from "@/mod";
import {
	type WorkflowBranchContextOf,
	type WorkflowContextOf,
	type WorkflowLoopContextOf,
	type WorkflowStepContextOf,
	workflow,
} from "@/workflow/mod";

describe("ActorDefinition", () => {
	describe("schema config types", () => {
		it("events do not accept queue-style schemas", () => {
			actor({
				state: {},
				events: {
					// @ts-expect-error events must use primitive schemas, not queue definitions.
					invalid: queue<{ foo: string }, { ok: true }>(),
				},
				actions: {},
			});
		});
	});

	describe("ActorContextOf type utility", () => {
		it("should correctly extract the context type from an ActorDefinition", () => {
			// Define some simple types for testing
			interface TestState {
				counter: number;
			}

			interface TestConnParams {
				clientId: string;
			}

			interface TestConnState {
				lastSeen: number;
			}

			interface TestVars {
				foo: string;
			}

			interface TestInput {
				bar: string;
			}

			interface TestDatabase {
				createClient: (ctx: DatabaseProviderContext) => Promise<{ execute: (query: string) => any }>;
				onMigrate: () => void;
			}

			// For testing type utilities, we don't need a real actor instance
			// We just need a properly typed ActorDefinition to check against
			type TestActions = Record<never, never>;
			const dummyDefinition = {} as ActorDefinition<
				TestState,
				TestConnParams,
				TestConnState,
				TestVars,
				TestInput,
				TestDatabase,
				Record<never, never>,
				Record<never, never>,
				TestActions
			>;

			// Use expectTypeOf to verify our type utility works correctly
			expectTypeOf<
				ActorContextOf<typeof dummyDefinition>
			>().toEqualTypeOf<
				ActorContext<
					TestState,
					TestConnParams,
					TestConnState,
					TestVars,
					TestInput,
					TestDatabase,
					Record<never, never>,
					Record<never, never>
				>
			>();

			// Make sure that different types are not compatible
			interface DifferentState {
				value: string;
			}

			expectTypeOf<
				ActorContextOf<typeof dummyDefinition>
			>().not.toEqualTypeOf<
				ActorContext<
					DifferentState,
					TestConnParams,
					TestConnState,
					TestVars,
					TestInput,
					TestDatabase,
					Record<never, never>,
					Record<never, never>
				>
			>();
		});
	});

	describe("queue type inference", () => {
		const queueTypeActor = actor({
			state: {},
			queues: {
				foo: queue<{ fooBody: string }>(),
				bar: queue<{ barBody: number }>(),
				completable: queue<{ input: string }, { output: string }>(),
			},
			actions: {},
		});

		type QueueTypeContext = ActorContextOf<typeof queueTypeActor>;

		async function receiveFooBar(c: QueueTypeContext) {
			return await c.queue.next({
				names: ["foo", "bar"] as const,
			});
		}

		async function receiveCompletableManual(c: QueueTypeContext) {
			return await c.queue.next({
				names: ["completable"] as const,
				completable: true,
			});
		}

		async function receiveFromAllQueues(c: QueueTypeContext) {
			for await (const message of c.queue.iter()) {
				return message;
			}

			throw new Error("queue iteration terminated unexpectedly");
		}

		it("narrows message body by queue name", () => {
			type ReceivedFooBar = Awaited<ReturnType<typeof receiveFooBar>>[number];
			type FooBody = Extract<ReceivedFooBar, { name: "foo" }>["body"];
			type BarBody = Extract<ReceivedFooBar, { name: "bar" }>["body"];

			expectTypeOf<FooBody>().toEqualTypeOf<{ fooBody: string }>();
			expectTypeOf<BarBody>().toEqualTypeOf<{ barBody: number }>();
		});

		it("completable queue messages expose correctly typed complete()", () => {
			type ManualMessage = Awaited<
				ReturnType<typeof receiveCompletableManual>
			>[number];
			type CompleteArgs = ManualMessage extends {
				complete: (...args: infer TArgs) => Promise<void>;
			}
				? TArgs
				: never;

			expectTypeOf<CompleteArgs>().toEqualTypeOf<
				[response: { output: string }]
			>();
		});

		it("infers queue body types when iterating c.queue.iter()", () => {
			type Received = Awaited<ReturnType<typeof receiveFromAllQueues>>;
			type FooBody = Extract<Received, { name: "foo" }>["body"];
			type BarBody = Extract<Received, { name: "bar" }>["body"];
			type CompletableBody = Extract<
				Received,
				{ name: "completable" }
			>["body"];

			expectTypeOf<FooBody>().toEqualTypeOf<{ fooBody: string }>();
			expectTypeOf<BarBody>().toEqualTypeOf<{ barBody: number }>();
			expectTypeOf<CompletableBody>().toEqualTypeOf<{ input: string }>();
		});
	});

	describe("workflow context type inference", () => {
		it("infers queue and event types for workflow ctx", () => {
			actor({
				state: {},
				queues: {
					foo: queue<{ fooBody: string }>(),
					bar: queue<{ barBody: number }>(),
				},
				events: {
					updated: event<{ count: number }>(),
					pair: event<[number, string]>(),
				},
				run: workflow(async (ctx) => {
					const single = await ctx.queue.next("wait-single", {
						names: ["foo"] as const,
					});
					if (single.name === "foo") {
						expectTypeOf(single.body).toEqualTypeOf<{
							fooBody: string;
						}>();
					}

					const union = await ctx.queue.next("wait-union", {
						names: ["foo", "bar"],
					});
					if (union.name === "foo") {
						expectTypeOf(union.body).toEqualTypeOf<{
							fooBody: string;
						}>();
					}
					if (union.name === "bar") {
						expectTypeOf(union.body).toEqualTypeOf<{
							barBody: number;
						}>();
					}

					ctx.broadcast("updated", { count: 1 });
					ctx.broadcast("pair", 1, "ok");
					// @ts-expect-error wrong payload shape
					ctx.broadcast("updated", { count: "no" });
					// @ts-expect-error unknown event name
					ctx.broadcast("missing", { count: 1 });
				}),
				actions: {},
			});
		});

		it("mirrors queue name/completable typing for workflow queue.next and queue.nextBatch", () => {
			actor({
				state: {},
				queues: {
					foo: queue<{ fooBody: string }>(),
					bar: queue<{ barBody: number }>(),
					completable: queue<{ input: string }, { output: string }>(),
				},
				run: workflow(async (ctx) => {
					const message = await ctx.queue.next("wait-completable", {
						names: ["completable"] as const,
						completable: true,
					});
					if (message.name === "completable") {
						expectTypeOf(message.body).toEqualTypeOf<{ input: string }>();
						type CompleteArgs = Parameters<typeof message.complete>;
						expectTypeOf<CompleteArgs>().toEqualTypeOf<
							[response: { output: string }]
						>();
					}

					const batch = await ctx.queue.nextBatch("wait-batch", {
						names: ["foo", "bar"] as const,
						count: 2,
					});
					type BatchMessage = (typeof batch)[number];
					type FooBody = Extract<BatchMessage, { name: "foo" }>["body"];
					type BarBody = Extract<BatchMessage, { name: "bar" }>["body"];
					expectTypeOf<FooBody>().toEqualTypeOf<{ fooBody: string }>();
					expectTypeOf<BarBody>().toEqualTypeOf<{ barBody: number }>();
				}),
				actions: {},
			});
		});

		it("does not require explicit queue.next body generic for single-queue actors", () => {
			type Decision = { approved: boolean; approver: string };
			actor({
				state: {},
				queues: {
					decision: queue<Decision>(),
				},
				run: workflow(async (ctx) => {
					const message = await ctx.queue.next("wait-decision", {
						names: ["decision"],
					});
					expectTypeOf(message.body).toEqualTypeOf<Decision>();
				}),
				actions: {},
			});
		});

		it("supports Workflow*ContextOf helpers for standalone workflow step functions", () => {
			const workflowHelperActor = actor({
				state: {
					count: 0,
				},
				queues: {
					work: queue<{ delta: number }>(),
				},
				run: workflow(async (ctx) => {
					await ctx.step("root-helper", async () => {
						applyRootHelper(ctx);
					});

					await ctx.loop("loop-helper", async (loopCtx) => {
						const message = await loopCtx.queue.next("wait-work", {
							names: ["work"] as const,
						});
						await loopCtx.step("apply-loop", async () => {
							applyLoopHelper(loopCtx, message.body.delta);
						});

						await loopCtx.join("branch-helper", {
							one: {
								run: async (branchCtx) => {
									await branchCtx.step("apply-branch", async () => {
										applyBranchHelper(branchCtx);
									});
									return 1;
								},
							},
						});

						await loopCtx.step("apply-step", async () => {
							applyStepHelper(loopCtx);
						});
					});
				}),
				actions: {},
			});

			function applyRootHelper(c: WorkflowContextOf<typeof workflowHelperActor>): void {
				expectTypeOf(c.state.count).toEqualTypeOf<number>();
			}

			function applyLoopHelper(
				c: WorkflowLoopContextOf<typeof workflowHelperActor>,
				delta: number,
			): void {
				c.state.count += delta;
			}

			function applyBranchHelper(
				c: WorkflowBranchContextOf<typeof workflowHelperActor>,
			): void {
				c.state.count += 1;
			}

			function applyStepHelper(
				c: WorkflowStepContextOf<typeof workflowHelperActor>,
			): void {
				c.state.count += 1;
			}

			expectTypeOf<
				WorkflowLoopContextOf<typeof workflowHelperActor>
			>().toEqualTypeOf<WorkflowContextOf<typeof workflowHelperActor>>();
			expectTypeOf<
				WorkflowBranchContextOf<typeof workflowHelperActor>
			>().toEqualTypeOf<WorkflowContextOf<typeof workflowHelperActor>>();
			expectTypeOf<
				WorkflowStepContextOf<typeof workflowHelperActor>
			>().toEqualTypeOf<WorkflowContextOf<typeof workflowHelperActor>>();
			expectTypeOf<
				WorkflowContextOfFromRoot<typeof workflowHelperActor>
			>().toEqualTypeOf<WorkflowContextOf<typeof workflowHelperActor>>();
		});
	});

	describe("database type inference", () => {
		it("supports typed rows for c.db.execute", () => {
			actor({
				state: {},
				db: db(),
				actions: {
					readFoo: async (c) => {
						const rows = await c.db.execute<{ foo: string }>(
							"SELECT foo FROM bar",
						);
						expectTypeOf(rows).toEqualTypeOf<Array<{ foo: string }>>();
					},
				},
			});
		});
	});
});
