import { OpenAiClient, OpenAiLanguageModel } from "@effect/ai-openai";
import { Config, Effect, Layer } from "effect";
import { FetchHttpClient } from "effect/http";

const OPENAI_API_URL = "https://api.openai.com/v1";

// The LLM is wired as a Layer so it can be swapped without touching the actor.
// The actor's action handlers require the `LanguageModel` service; this Layer
// satisfies that requirement with a real OpenAI model.
//
// `LanguageModel` is produced by `OpenAiLanguageModel.layer`, which needs an
// `OpenAiClient`, which in turn needs an `HttpClient`. Composing the three with
// `Layer.provide` yields a single `Layer<LanguageModel>`.
//
// `OPENAI_API_KEY` is read from config. `OPENAI_BASE_URL` is optional: leave it
// unset to hit the real OpenAI API, or point it at any OpenAI-compatible
// endpoint (this is the same knob the test uses to target a mock server).
export const OpenAiModelLayer = Layer.unwrap(
	Effect.gen(function* () {
		const apiUrl = yield* Config.String("OPENAI_BASE_URL").pipe(
			Config.withDefault(OPENAI_API_URL),
		);
		// Custom gateways have their own model availability and retirement dates.
		const model =
			apiUrl.replace(/\/+$/, "") === OPENAI_API_URL
				? "gpt-6-luna"
				: "gpt-4o-mini";

		return OpenAiLanguageModel.layer({ model }).pipe(
			Layer.provide(
				OpenAiClient.layerConfig({
					apiKey: Config.Redacted("OPENAI_API_KEY"),
					apiUrl: Config.succeed(apiUrl),
				}),
			),
			Layer.provide(FetchHttpClient.layer),
		);
	}),
);
