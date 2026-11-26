import { NodeRuntime } from "@effect/platform-node";
import { Client, Registry } from "@rivetkit/effect";
import { Layer } from "effect";
import { CounterLive } from "./actors/counter/live.ts";
import { TelemetryLayer } from "./telemetry.ts";

const endpoint = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";

const ActorsLayer = CounterLive.pipe(Layer.provide(Client.layer({ endpoint })));

/**
 * With no `RIVET_ENDPOINT`, RivetKit spawns a local rivet-engine process on
 * http://127.0.0.1:6420 when `RIVET_RUN_ENGINE=1` is set, as `pnpm dev` and
 * `pnpm start` do, or when `NODE_ENV` is not `production`. To use an engine
 * that already runs, set `RIVET_ENDPOINT` and do not set `RIVET_RUN_ENGINE`.
 * `RIVET_ENGINE_BINARY` is the path of the engine binary to spawn, for
 * example a `cargo build` binary.
 */
const MainLayer = Registry.serve(ActorsLayer).pipe(
	Layer.provide(Registry.layer()),
	Layer.provide(TelemetryLayer),
);

// Keeps the layer alive. Tears down on SIGINT/SIGTERM.
Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
