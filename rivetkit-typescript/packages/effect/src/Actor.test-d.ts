import {
	Context,
	Effect,
	type Layer,
	Schema,
	SchemaTransformation,
} from "effect";
import type { RawAccess } from "rivetkit/db";
import { db } from "rivetkit/db";
import { describe, expectTypeOf, test } from "vitest";
import * as Action from "./Action";
import * as Actor from "./Actor";
import * as ActorState from "./ActorState";
import type * as Client from "./Client";
import type * as State from "./State";

class SomeDep extends Context.Service<SomeDep, { readonly x: number }>()(
	"SomeDep",
) {}

const TestActor = Actor.make("TestActor", {
	actions: [Action.make("GetContext")],
});

type TestActions = (typeof TestActor.actions)[number];

const TestState = ActorState.make("TestState", {
	schema: Schema.Struct({
		count: Schema.Number,
	}),
	initialValue: () => ({ count: 0 }),
});

const TagsCsv = Schema.String.pipe(
	Schema.decodeTo(
		Schema.Array(Schema.String),
		SchemaTransformation.transform({
			decode: (s: string): ReadonlyArray<string> => s.split(","),
			encode: (arr: ReadonlyArray<string>) => arr.join(","),
		}),
	),
);

const TransformedState = ActorState.make("TransformedState", {
	schema: Schema.Struct({
		when: Schema.DateFromString,
		url: Schema.URLFromString,
		id: Schema.BigIntFromString,
		bytes: Schema.Uint8ArrayFromBase64,
		tags: TagsCsv,
		history: Schema.Array(
			Schema.Struct({
				at: Schema.DateFromString,
				payload: Schema.Uint8ArrayFromBase64,
			}),
		),
	}),
	initialValue: () => ({
		when: new Date("2024-01-15T10:30:00.000Z"),
		url: new URL("https://rivet.dev/docs"),
		id: 1n,
		bytes: new Uint8Array([1, 2, 3]),
		tags: ["alpha", "beta"],
		history: [
			{
				at: new Date("2024-01-15T10:30:00.000Z"),
				payload: new Uint8Array([4, 5, 6]),
			},
		],
	}),
});

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
		expectTypeOf(TestActor.toLayer).toBeCallableWith(
			(_wakeOptions: any) => ({
				GetContext: () => Effect.void,
			}),
		);
	});

	test("wake options omit state without a configured state type", () => {
		TestActor.toLayer((wakeOptions) => {
			// @ts-expect-error: stateless actors do not expose wakeOptions.state
			wakeOptions.state;
			expectTypeOf(
				wakeOptions.rawRivetkitContext.state,
			).toEqualTypeOf<never>();

			return {
				GetContext: () => Effect.void,
			};
		});
	});

	test("wake options carry the configured state type", () => {
		TestActor.toLayer(
			(wakeOptions) => {
				expectTypeOf(wakeOptions.state).toEqualTypeOf<
					State.State<{ readonly count: number }, Schema.SchemaError>
				>();

				return {
					GetContext: () => Effect.void,
				};
			},
			{ state: TestState },
		);
	});

	test("wake options carry the transformed state type", () => {
		TestActor.toLayer(
			(wakeOptions) => {
				expectTypeOf(wakeOptions.state).toEqualTypeOf<
					State.State<
						{
							readonly when: Date;
							readonly url: URL;
							readonly id: bigint;
							readonly bytes: Uint8Array;
							readonly tags: ReadonlyArray<string>;
							readonly history: ReadonlyArray<{
								readonly at: Date;
								readonly payload: Uint8Array;
							}>;
						},
						Schema.SchemaError
					>
				>();

				return {
					GetContext: () => Effect.void,
				};
			},
			{ state: TransformedState },
		);
	});

	test("wake options carry the raw RivetKit context with the encoded configured state type", () => {
		TestActor.toLayer(
			(wakeOptions) => {
				expectTypeOf(
					wakeOptions.rawRivetkitContext.state,
				).toEqualTypeOf<{ readonly count: number }>();

				return {
					GetContext: () => Effect.void,
				};
			},
			{ state: TestState },
		);
	});

	test("wake options carry the raw RivetKit context with the encoded transformed state type", () => {
		TestActor.toLayer(
			(wakeOptions) => {
				expectTypeOf(
					wakeOptions.rawRivetkitContext.state,
				).toEqualTypeOf<{
					readonly when: string;
					readonly url: string;
					readonly id: string;
					readonly bytes: string;
					readonly tags: string;
					readonly history: ReadonlyArray<{
						readonly at: string;
						readonly payload: string;
					}>;
				}>();

				return {
					GetContext: () => Effect.void,
				};
			},
			{ state: TransformedState },
		);
	});

	test("wake options carry the configured database client type", () => {
		TestActor.toLayer(
			(wakeOptions) => {
				expectTypeOf(
					wakeOptions.rawRivetkitContext.db,
				).toEqualTypeOf<RawAccess>();

				return {
					GetContext: () => Effect.void,
				};
			},
			{ db: db() },
		);
	});

	test("accepts a function returning an Effect of action handlers", () => {
		expectTypeOf(TestActor.toLayer).toBeCallableWith((_wakeOptions: any) =>
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
