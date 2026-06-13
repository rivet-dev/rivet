import { assert, layer } from "@effect/vitest";
import { Registry } from "@rivetkit/effect";
import { Effect, Layer } from "effect";
import { Counter } from "../src/actors/counter/api.ts";
import { CounterLive } from "../src/actors/counter/live.ts";

// `Registry.test` boots the actor in-process against a local engine. With no
// endpoint configured on `Registry.layer`, it auto-spawns a `rivet-engine` for
// the duration of the suite, the same way `setupTest` does for the other
// examples. It also provides `Client`, so `Counter.client` resolves here.
const TestLayer = Registry.test.pipe(
	Layer.provideMerge(CounterLive),
	Layer.provide(Registry.layer()),
);

layer(TestLayer)("hello-world-effect", (it) => {
	it.effect("increments and reads the count back", () =>
		Effect.gen(function* () {
			const counter = (yield* Counter.client).getOrCreate("t-increment");
			assert.strictEqual(yield* counter.Increment({ amount: 1 }), 1);
			assert.strictEqual(yield* counter.Increment({ amount: 5 }), 6);
			assert.strictEqual(yield* counter.GetCount(), 6);
		}),
	);

	it.effect("isolates state across keys", () =>
		Effect.gen(function* () {
			const client = yield* Counter.client;
			const a = client.getOrCreate("t-iso-a");
			const b = client.getOrCreate("t-iso-b");
			yield* a.Increment({ amount: 2 });
			yield* b.Increment({ amount: 7 });
			assert.strictEqual(yield* a.GetCount(), 2);
			assert.strictEqual(yield* b.GetCount(), 7);
		}),
	);
});
