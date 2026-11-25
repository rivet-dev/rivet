import { Action, Actor } from "@rivetkit/effect";
import { Context, Effect, Layer, Schema } from "effect";

export class Translator extends Context.Service<
	Translator,
	{
		readonly translate: (text: string) => Effect.Effect<string>;
	}
>()("Translator") {}

export const TranslatorLive = Layer.succeed(
	Translator,
	Translator.of({
		translate: (text) => Effect.succeed(text.replace("Hello", "Hallo")),
	}),
);

export const Greet = Action.make("Greet", {
	payload: { name: Schema.String },
	success: Schema.String,
});

export const Greeter = Actor.make("Greeter", {
	actions: [Greet],
});

export const GreeterLive = Greeter.toLayer(
	Effect.gen(function* () {
		const translator = yield* Translator;

		return Greeter.of({
			Greet: ({ payload }) =>
				translator.translate(`Hello, ${payload.name}`),
		});
	}),
);

// The actor layer requires `Translator` until you provide it.
export const ActorsLayer = GreeterLive.pipe(Layer.provide(TranslatorLive));
