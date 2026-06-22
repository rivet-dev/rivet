import { NodeRuntime } from "@effect/platform-node";
import { Client, Registry } from "@rivetkit/effect";
import { Layer } from "effect";
import { AgentLive } from "./actors/agent/live.ts";
import { TelemetryLayer } from "./telemetry.ts";
import { OpenAiModelLayer } from "./model.ts";

const endpoint = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";

// Provide the LLM model Layer (`OpenAiModelLayer`) and the client transport to
// the actor layer. Swapping `OpenAiModelLayer` for another `Layer<LanguageModel>`
// is all it takes to change providers; the actor code never changes.
const ActorsLayer = AgentLive.pipe(
	Layer.provide(OpenAiModelLayer),
	Layer.provide(Client.layer({ endpoint })),
);

// Engine config defaults to spawning a local rivet-engine process and
// listening on http://127.0.0.1:6420 (override via RIVET_ENDPOINT to
// point at a remote engine). For dev builds without a packaged engine,
// set RIVET_ENGINE_BINARY to the path of a `cargo build` binary, e.g.:
//   RIVET_ENGINE_BINARY=$(pwd)/target/debug/rivet-engine pnpm start
const MainLayer = Registry.serve(ActorsLayer).pipe(
	Layer.provide(Registry.layer()),
	Layer.provide(TelemetryLayer),
);

// Keeps the layer alive. Tears down on SIGINT/SIGTERM.
Layer.launch(MainLayer).pipe(NodeRuntime.runMain);

// Or create a web handler, which can be used in serverless environments.
export const { handler, dispose } = Registry.toWebHandler(
	ActorsLayer.pipe(
		Layer.provideMerge(Registry.layer()),
		Layer.provide(TelemetryLayer),
	),
);
