import { LLMock } from "@copilotkit/llmock";
import { OpenAiClient, OpenAiLanguageModel } from "@effect/ai-openai";
import { assert, layer } from "@effect/vitest";
import { Registry } from "@rivetkit/effect";
import { Effect, Layer, Random, Redacted } from "effect";
import { FetchHttpClient } from "effect/unstable/http";
import { inject } from "vitest";
import { Agent } from "../src/actors/agent/api.ts";
import { AgentLive } from "../src/actors/agent/live.ts";
import { prepareNamespace, waitForEnvoy } from "./shared-engine.ts";

// Per repo rules there is no module mocking: a real OpenAI-compatible mock LLM
// server runs in-process and the Effect AI provider is pointed at its base URL.
//
// `@effect/ai-openai` (effect 4 beta) speaks the OpenAI Responses API
// (`POST /v1/responses`), so we mount a small real HTTP handler that returns a
// Responses-shaped payload. The agent's memory is proven by sending two turns
// and asserting both the reply and the persisted history.
const replies: ReadonlyArray<{ pattern: string; content: string }> = [
	{ pattern: "who are you", content: "I am a mock assistant." },
	{ pattern: "remember", content: "You said: remember this." },
];

const mock = new LLMock({ port: 0 });
mock.mount("/v1/responses", {
	async handleRequest(req, res) {
		if (req.method !== "POST") return false;

		const chunks: Buffer[] = [];
		for await (const chunk of req) chunks.push(chunk as Buffer);
		const body = JSON.parse(Buffer.concat(chunks).toString() || "{}");

		const text = lastUserText(body.input ?? []);
		const match = replies.find((reply) =>
			text.toLowerCase().includes(reply.pattern),
		);
		const content = match?.content ?? "I'm not sure how to answer that.";

		res.writeHead(200, { "content-type": "application/json" });
		res.end(
			JSON.stringify({
				id: "resp-mock",
				object: "response",
				created_at: 0,
				model: body.model,
				status: "completed",
				output: [
					{
						type: "message",
						id: "msg-mock",
						status: "completed",
						role: "assistant",
						// `annotations` is required by the beta OpenAI Responses
						// parser; real OpenAI always includes it.
						content: [
							{
								type: "output_text",
								text: content,
								annotations: [],
							},
						],
					},
				],
				usage: { input_tokens: 0, output_tokens: 0, total_tokens: 0 },
			}),
		);
		return true;
	},
});
await mock.start();

// Extracts the latest user message text from a Responses API `input` array.
// User content arrives as an array of `{ type: "input_text", text }` parts;
// system content is a plain string.
function lastUserText(
	input: ReadonlyArray<{ role: string; content: unknown }>,
): string {
	const lastUser = [...input].reverse().find((m) => m.role === "user");
	if (lastUser === undefined) return "";
	if (typeof lastUser.content === "string") return lastUser.content;
	if (Array.isArray(lastUser.content)) {
		return lastUser.content
			.map((part) => (typeof part?.text === "string" ? part.text : ""))
			.join(" ");
	}
	return "";
}

// Swap-in mock model Layer. It is exactly the production wiring from
// `src/model.ts`, except `apiUrl` targets the mock server instead of OpenAI.
// This is the dependency-injection seam: the actor is unchanged; only the
// `LanguageModel` Layer differs between dev and test.
const MockModelLayer = OpenAiLanguageModel.layer({ model: "gpt-4o-mini" }).pipe(
	Layer.provide(
		OpenAiClient.layer({
			apiKey: Redacted.make("test-key"),
			apiUrl: `${mock.url}/v1`,
		}),
	),
	Layer.provide(FetchHttpClient.layer),
);

// Talk to the shared engine (spawned in globalSetup) against a unique namespace
// + runner pool so envoy registrations can't bleed across runs.
const { endpoint, token, namespace, poolName } = await prepareNamespace(
	inject("rivetEngine").endpoint,
);

// Block until the in-process envoy has registered against the engine's pool;
// `Registry.test`'s `.start()` returns before that round-trip completes.
const ReadyForEnvoy = Layer.effectDiscard(
	Effect.tryPromise(() => waitForEnvoy(endpoint, namespace, poolName)).pipe(
		Effect.orDie,
	),
);

const TestLayer = ReadyForEnvoy.pipe(
	Layer.provideMerge(
		Registry.test.pipe(
			Layer.provideMerge(AgentLive.pipe(Layer.provide(MockModelLayer))),
			Layer.provide(Registry.layer({ endpoint, token, namespace })),
		),
	),
);

const freshAgent = Effect.gen(function* () {
	const client = yield* Agent.client;
	return client.getOrCreate(`agent_${yield* Random.nextUUIDv4}`);
});

layer(TestLayer, { timeout: 30_000 })("agent-effect", (it) => {
	it.effect("replies via the LLM and persists conversation history", () =>
		Effect.gen(function* () {
			const agent = yield* freshAgent;

			const first = yield* agent.SendMessage({
				content: "Hello, who are you?",
			});
			assert.strictEqual(first, "I am a mock assistant.");

			const second = yield* agent.SendMessage({
				content: "Please remember this.",
			});
			assert.strictEqual(second, "You said: remember this.");

			// History accumulates across calls: two user turns + two assistant
			// turns, in order.
			const history = yield* agent.GetHistory();
			assert.deepStrictEqual(
				history.map((turn) => turn.role),
				["user", "assistant", "user", "assistant"],
			);
			assert.strictEqual(history[0].content, "Hello, who are you?");
			assert.strictEqual(history[1].content, "I am a mock assistant.");
		}),
	);

	it.effect("rejects empty messages with a typed error", () =>
		Effect.gen(function* () {
			const agent = yield* freshAgent;

			const exit = yield* agent
				.SendMessage({ content: "   " })
				.pipe(Effect.flip, Effect.exit);

			assert.isTrue(exit._tag === "Success");
			if (exit._tag === "Success") {
				assert.strictEqual(exit.value._tag, "EmptyMessageError");
			}
		}),
	);
});
