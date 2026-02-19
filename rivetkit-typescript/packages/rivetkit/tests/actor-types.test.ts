import { describe, expectTypeOf, it } from "vitest";
import { actor, event, queue } from "@/actor/mod";
import type { ActorContext, ActorContextOf } from "@/actor/contexts";
import type { ActorDefinition } from "@/actor/definition";
import type { DatabaseProviderContext } from "@/db/config";
import { workflow } from "@/workflow/mod";

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
					const [single] = await ctx.queue.next("wait-single", {
						names: ["foo"] as const,
					});
					if (single && single.name === "foo") {
						expectTypeOf(single.body).toEqualTypeOf<{
							fooBody: string;
						}>();
					}

					const [union] = await ctx.queue.next("wait-union", {
						names: ["foo", "bar"],
					});
					if (union?.name === "foo") {
						expectTypeOf(union.body).toEqualTypeOf<{
							fooBody: string;
						}>();
					}
					if (union?.name === "bar") {
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

		it("does not require explicit queue.next body generic for single-queue actors", () => {
			type Decision = { approved: boolean; approver: string };
			actor({
				state: {},
				queues: {
					decision: queue<Decision>(),
				},
				run: workflow(async (ctx) => {
					const [message] = await ctx.queue.next("wait-decision", {
						names: ["decision"],
					});
					if (message) {
						expectTypeOf(message.body).toEqualTypeOf<Decision>();
					}
				}),
				actions: {},
			});
		});
	});
});
