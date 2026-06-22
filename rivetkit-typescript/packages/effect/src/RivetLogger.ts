import { Effect, Logger, Layer } from "effect";
import type { Logger as PinoLogger } from "rivetkit/log";
import {
	BaseLogger,
	getOrCreateBaseLogger,
	makeEffectLogger,
} from "./internal/logging.ts";

/**
 * Builds an Effect logger from a custom Pino-compatible logger.
 *
 * Use this with Effect's `Logger.layer` when you only need to customize Effect
 * log output. Use {@link layerFromPino} when RivetKit's underlying TypeScript SDK
 * logs should use the same logger too.
 *
 * @example
 * ```ts
 * import { Logger } from "effect"
 * import { RivetLogger } from "@rivetkit/effect"
 * import { pino } from "pino"
 *
 * const baseLogger = pino({ transport: { target: "pino-pretty" } })
 * const LoggerLive = Logger.layer([RivetLogger.fromPino(baseLogger)])
 * ```
 */
export const fromPino = (
	baseLogger: PinoLogger,
): Logger.Logger<unknown, void> => makeEffectLogger(baseLogger);

/**
 * Builds a logging layer from a custom Pino-compatible logger.
 *
 * The layer installs the matching Effect logger and configures the underlying
 * RivetKit TypeScript SDK logs to go through the same logger. The Effect tracer
 * logger is installed alongside the Pino adapter so log events remain attached
 * to active traces.
 */
export const layerFromPino = (
	baseLogger: PinoLogger,
	options?: {
		readonly mergeWithExisting?: boolean | undefined;
	},
): Layer.Layer<BaseLogger> =>
	Layer.mergeAll(
		Layer.succeed(BaseLogger, baseLogger),
		Logger.layer([Logger.tracerLogger, fromPino(baseLogger)], {
			mergeWithExisting: options?.mergeWithExisting,
		}),
	);

/**
 * Default RivetKit Effect logging layer.
 *
 * The layer creates a base logger from `References.MinimumLogLevel` and installs
 * the Effect logger adapter. Applications that want custom formatting or
 * transports should provide {@link layerFromPino} instead.
 */
export const defaultLayer: Layer.Layer<never> = Layer.unwrap(
	Effect.map(getOrCreateBaseLogger, layerFromPino),
);
