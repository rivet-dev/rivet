import { NodeRuntime } from "@effect/platform-node";
import { Registry } from "@rivetkit/effect";
import { Layer } from "effect";
import { GreeterLive, TranslatorLive } from "../actors-effect/services";
import { CounterLive } from "../actors-quickstart-effect/src/live";

const ActorsLayer = Layer.mergeAll(
	CounterLive,
	GreeterLive.pipe(Layer.provide(TranslatorLive)),
);

const MainLayer = Registry.serve(ActorsLayer).pipe(
	Layer.provide(
		Registry.layer({
			endpoint: "https://api.rivet.dev",
			namespace: "my-namespace",
			token: process.env.RIVET_TOKEN,
			noWelcome: true,
		}),
	),
);

Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
