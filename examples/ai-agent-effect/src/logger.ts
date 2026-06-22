import { RivetLogger } from "@rivetkit/effect";
import { pino } from "pino";

// This layer adds a custom Pino logger for Effect.log* calls and the underlying
// RivetKit logs while keeping Effect log events attached to traces.
export const PrettyLoggerLayer = RivetLogger.layerFromPino(
	pino({ transport: { target: "pino-pretty" } }),
);
