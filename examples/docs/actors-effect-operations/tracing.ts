import { NodeHttpClient, NodeRuntime } from "@effect/platform-node";
import { Registry } from "@rivetkit/effect";
import { Layer } from "effect";
import { OtlpSerialization, OtlpTracer } from "effect/observability";
import { CounterLive } from "../actors-quickstart-effect/src/live";

const TracingLayer = OtlpTracer.layer({
	url: "http://localhost:4318/v1/traces",
	resource: { serviceName: "counter" },
}).pipe(
	Layer.provide(OtlpSerialization.layerJson),
	Layer.provide(NodeHttpClient.layerUndici),
);

const MainLayer = Registry.serve(CounterLive).pipe(
	Layer.provide(Registry.layer()),
	Layer.provide(TracingLayer),
);

Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
