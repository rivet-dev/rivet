import { NodeRuntime } from "@effect/platform-node";
import { Registry } from "@rivetkit/effect";
import { Layer } from "effect";
import { CounterLive } from "./live";

const MainLayer = Registry.serve(CounterLive).pipe(
	Layer.provide(Registry.layer()),
);

Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
