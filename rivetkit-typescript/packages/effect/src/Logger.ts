import { Effect, Logger as EffectLogger, Layer } from "effect";
import type { Logger as PinoLogger } from "rivetkit/log";
import {
	BaseLogger,
	getOrCreateBaseLogger,
	makeEffectLogger,
} from "./internal/logging.ts";

/**
 * Builds a logging layer from a custom Pino-compatible logger.
 *
 * The layer installs the matching Effect logger and configures the underlying
 * RivetKit TypeScript SDK logs to go through the same logger.
 *
 * @example
 * ```ts
 * import { Logger } from "@rivetkit/effect"
 * import { pino } from "pino"
 *
 * const LoggerLive = Logger.layerPino(
 *   pino({ transport: { target: "pino-pretty" } })
 * )
 * ```
 */
export const layerPino = (baseLogger: PinoLogger) =>
	Layer.mergeAll(
		Layer.succeed(BaseLogger, baseLogger),
		EffectLogger.layer([
			EffectLogger.tracerLogger,
			makeEffectLogger(baseLogger),
		]),
	);

/**
 * Default RivetKit Effect logging layer.
 *
 * The layer creates a base logger from `References.MinimumLogLevel` and installs
 * the Effect logger adapter. Applications that want custom formatting or
 * transports should provide {@link layerPino} instead.
 */
export const layer: Layer.Layer<never> = Layer.unwrap(
	Effect.map(getOrCreateBaseLogger, layerPino),
);
