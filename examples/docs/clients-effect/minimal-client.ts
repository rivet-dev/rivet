import { NodeRuntime } from "@effect/platform-node";
import { Client } from "@rivetkit/effect";
import { Effect } from "effect";
import { Counter } from "./api";

const program = Effect.gen(function* () {
	const counter = (yield* Counter.client).getOrCreate("my-counter");

	const count = yield* counter.Increment({ amount: 1 });
	yield* Effect.log(`New count: ${count}`);
});

program.pipe(
	Effect.provide(Client.layer({ endpoint: "http://localhost:6420" })),
	NodeRuntime.runMain,
);
