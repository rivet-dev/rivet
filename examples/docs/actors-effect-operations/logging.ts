import { NodeRuntime } from "@effect/platform-node";
import { Registry, RivetLogger } from "@rivetkit/effect";
import { Layer } from "effect";
import { pino } from "pino";
import { CounterLive } from "../actors-quickstart-effect/src/live";

const LoggerLayer = RivetLogger.layerFromPino(pino({ level: "debug" }));

const MainLayer = Registry.serve(CounterLive).pipe(
	Layer.provide(Registry.layer()),
	Layer.provide(LoggerLayer),
);

Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
