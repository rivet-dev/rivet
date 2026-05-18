import { Context, Effect, type Layer } from "effect";
import { describe, expectTypeOf, test } from "vitest";
import * as Action from "./Action";
import * as Actor from "./Actor";
import type * as Client from "./Client";

class SomeDep extends Context.Service<SomeDep, { readonly x: number }>()(
	"SomeDep",
) {}

const TestActor = Actor.make("TestActor", {
	actions: [Action.make("GetContext")],
});

type TestActions = (typeof TestActor.actions)[number];

describe("Actor.make", () => {
	test("preserves the name literal", () => {
		expectTypeOf(TestActor.name).toEqualTypeOf<"TestActor">();
	});
});

describe("Actor.make(...).toLayer", () => {
	test("is a function", () => {
		expectTypeOf(TestActor.toLayer).toBeFunction();
	});

	test("accepts a plain action handlers object", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith({
			GetContext: () => Effect.void,
		});
	});

	test("accepts an Effect of action handlers", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith(
			Effect.gen(function* () {
				return {
					GetContext: () => Effect.void,
				};
			}),
		);
	});

	test("accepts a function returning a plain action handlers object", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith((_wakeOptions) => ({
			GetContext: () => Effect.void,
		}));
	});

	test("accepts a function returning an Effect of action handlers", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith((_wakeOptions) =>
			Effect.gen(function* () {
				return {
					GetContext: () => Effect.void,
				};
			}),
		);
	});

	test("accepts an Effect that resolves to a wake function", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith(
			Effect.gen(function* () {
				// Allow for initialization logic before the per-entity wake function is called

				return (_wakeOptions: any) =>
					Effect.gen(function* () {
						return {
							GetContext: () => Effect.void,
						};
					});
			}),
		);
	});

	test("accepts an Effect.fn returning action handlers", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith(
			Effect.fn("wake")(function* (_wakeOptions) {
				return {
					GetContext: () => Effect.void,
				};
			}),
		);
	});

	test("returns a Layer", () => {
		expectTypeOf(TestActor.toLayer).returns.toExtend<Layer.Any>();
	});

	test("action handler's envelope is typed against the action", () => {
		TestActor.toLayer({
			GetContext: (envelope) => {
				expectTypeOf(envelope._tag).toEqualTypeOf<"GetContext">();
				expectTypeOf(envelope.action).toExtend<Action.Any>();
				return Effect.void;
			},
		});
	});

	test("missing action handler is rejected", () => {
		// @ts-expect-error: GetContext handler is required
		TestActor.toLayer({});
	});

	test.todo("unknown action handler key is rejected", () => {
		TestActor.toLayer({
			GetContext: () => Effect.void,
			// TODO: toLayer should reject unknown action handler keys
			Unknown: () => Effect.void,
		});
	});

	test.todo("wake-effect requirements surface in the Layer", () => {
		const layer = TestActor.toLayer(
			Effect.gen(function* () {
				yield* SomeDep;
				return { GetContext: () => Effect.void };
			}),
		);
		type Reqs =
			typeof layer extends Layer.Layer<any, any, infer R> ? R : never;
		// @ts-expect-error: TODO - expectTypeOf<T>() no-arg generic form not resolving
		expectTypeOf<SomeDep>().toExtend<Reqs>();
	});
});

describe("Actor.make(...).client", () => {
	test("yields a typed Accessor", () => {
		expectTypeOf(TestActor.client).toEqualTypeOf<
			Effect.Effect<Actor.Accessor<TestActions>, never, Client.Client>
		>();
	});
});
