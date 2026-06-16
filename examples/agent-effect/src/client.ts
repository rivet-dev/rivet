import { NodeRuntime } from "@effect/platform-node";
import { Client } from "@rivetkit/effect";
import { Effect } from "effect";
import { Agent } from "./actors/mod.ts";
import { PrettyLoggerLayer } from "./logger.ts";

const program = Effect.gen(function* () {
	// `Actor.client` yields a typed accessor backed by the Effect SDK client layer.
	const agentClient = yield* Agent.client;
	const agent = agentClient.getOrCreate("session-1");

	const first = yield* agent.SendMessage({ content: "Hello, who are you?" });
	yield* Effect.log(`assistant: ${first}`);

	const second = yield* agent.SendMessage({
		content: "What did I just ask you?",
	});
	yield* Effect.log(`assistant: ${second}`);

	// The declared error arrives as a real tagged instance, caught by tag.
	yield* agent
		.SendMessage({ content: "   " })
		.pipe(
			Effect.catchTag("EmptyMessageError", (err) =>
				Effect.log(`rejected: ${err.message}`),
			),
		);

	// The whole conversation is persisted in actor state.
	const history = yield* agent.GetHistory();
	yield* Effect.log(`history has ${history.length} turns`);
});

const ClientLayer = Client.layer({ endpoint: "http://127.0.0.1:6420" });

program
	.pipe(Effect.provide(ClientLayer), Effect.provide(PrettyLoggerLayer))
	.pipe(NodeRuntime.runMain);
