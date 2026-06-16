import { NodeRuntime } from "@effect/platform-node";
import { Client } from "@rivetkit/effect";
import { Effect } from "effect";
import { Counter } from "./actors/mod.ts";
import { PrettyLoggerLayer } from "./logger.ts";

const program = Effect.gen(function* () {
	// `Actor.client` yields a typed accessor backed by the Effect SDK client layer.
	const counterClient = yield* Counter.client;
	const counter = counterClient.getOrCreate("hello-world");

	const first = yield* counter.Increment({ amount: 1 });
	yield* Effect.log(`count is now ${first}`);

	const second = yield* counter.Increment({ amount: 5 });
	yield* Effect.log(`count is now ${second}`);

	// The declared error arrives as a real tagged instance, caught by tag.
	yield* counter
		.Increment({ amount: -1 })
		.pipe(
			Effect.catchTag("NegativeAmountError", (err) =>
				Effect.log(`rejected: ${err.message} (amount ${err.amount})`),
			),
		);

	const total = yield* counter.GetCount();
	yield* Effect.log(`final count: ${total}`);
});

const ClientLayer = Client.layer({ endpoint: "http://127.0.0.1:6420" });

program
	.pipe(Effect.provide(ClientLayer), Effect.provide(PrettyLoggerLayer))
	.pipe(NodeRuntime.runMain);
