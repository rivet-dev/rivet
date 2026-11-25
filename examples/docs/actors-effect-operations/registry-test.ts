import { Registry } from "@rivetkit/effect";
import { Effect, Layer } from "effect";
import { expect, test } from "vitest";
import { Counter } from "../actors-quickstart-effect/src/api";
import { CounterLive } from "../actors-quickstart-effect/src/live";

const TestLayer = Registry.test.pipe(
	Layer.provideMerge(CounterLive),
	Layer.provide(Registry.layer()),
);

test("increments the count", () =>
	Effect.gen(function* () {
		const counter = (yield* Counter.client).getOrCreate(
			crypto.randomUUID(),
		);

		yield* counter.Increment({ amount: 2 });
		const count = yield* counter.Increment({ amount: 3 });

		expect(count).toBe(5);
	}).pipe(Effect.provide(TestLayer), Effect.runPromise));
