import { Logger } from "@rivetkit/effect";
import { pino } from "pino";

// This layer replaces the default RivetKit Effect logger with a custom Pino
// logger. It affects both Effect.log* calls and the underlying RivetKit logs.
export const PrettyLoggerLayer = Logger.layerPino(
	pino({ transport: { target: "pino-pretty" } }),
);
