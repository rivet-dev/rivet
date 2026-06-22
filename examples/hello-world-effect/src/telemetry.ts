import { RivetLogger } from "@rivetkit/effect";
import { NodeHttpClient } from "@effect/platform-node";
import { Layer } from "effect";
import { OtlpSerialization, OtlpTracer } from "effect/unstable/observability";
import { pino } from "pino";

// This layer adds a custom Pino logger for Effect.log* calls and the underlying
// RivetKit logs while keeping Effect log events attached to traces.
export const PrettyLoggerLayer = RivetLogger.layerFromPino(
	pino({ transport: { target: "pino-pretty" } }),
);

export const OtlpTracingLayer = OtlpTracer.layer({
	url: "http://127.0.0.1:43110/ingest/otlp/v1/traces/rivet",
	resource: { serviceName: "example-chat-room-effect" },
}).pipe(
	Layer.provide(OtlpSerialization.layerJson),
	Layer.provide(NodeHttpClient.layerUndici),
);

export const TelemetryLayer = Layer.mergeAll(
	PrettyLoggerLayer,
	OtlpTracingLayer,
);
