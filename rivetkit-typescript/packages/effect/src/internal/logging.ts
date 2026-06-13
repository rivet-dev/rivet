import {
	Cause,
	Config,
	Context,
	Effect,
	Logger as EffectLogger,
	type LogLevel,
	References,
} from "effect";
import type * as Rivetkit from "rivetkit";
import {
	configureDefaultLogger,
	getBaseLogger,
	type Logger as PinoLogger,
	type LogLevel as PinoLogLevel,
} from "rivetkit/log";

const EMPTY_KEY = "/";
const KEY_SEPARATOR = "/";

type ActorLogContext = {
	readonly name: string;
	readonly key: Rivetkit.ActorKey;
	readonly actorId: string;
};

export class BaseLogger extends Context.Service<BaseLogger, PinoLogger>()(
	"@rivetkit/effect/Logger/BaseLogger",
) {}

const PinoLevelByEffectLevel: Record<LogLevel.LogLevel, PinoLogLevel> = {
	All: "trace",
	Trace: "trace",
	Debug: "debug",
	Info: "info",
	Warn: "warn",
	Error: "error",
	Fatal: "fatal",
	None: "silent",
};

export const toPinoLevel = (logLevel: LogLevel.LogLevel): PinoLogLevel =>
	PinoLevelByEffectLevel[logLevel];

const EffectLevelByPinoLevel: Record<PinoLogLevel, LogLevel.LogLevel> = {
	trace: "Trace",
	debug: "Debug",
	info: "Info",
	warn: "Warn",
	error: "Error",
	fatal: "Fatal",
	silent: "None",
};

const pinoLogLevelFromEnv = Config.string("RIVET_LOG_LEVEL").pipe(
	Config.map((value) => {
		const pinoLevel = value.toLowerCase();
		if (pinoLevel in EffectLevelByPinoLevel) {
			return EffectLevelByPinoLevel[pinoLevel as PinoLogLevel];
		}

		return "Info";
	}),
);

const logLevelFromEnv = Config.logLevel("RIVET_LOG_LEVEL").pipe(
	Config.orElse(() => pinoLogLevelFromEnv),
	Effect.option,
);

export const makeDefaultBaseLogger: Effect.Effect<PinoLogger> = Effect.gen(
	function* () {
		const context = yield* Effect.context();
		const providedMinimumLogLevel = Context.getOrUndefined(
			context,
			References.MinimumLogLevel,
		);
		const envLogLevel = yield* logLevelFromEnv;
		const logLevel =
			providedMinimumLogLevel !== undefined
				? providedMinimumLogLevel
				: envLogLevel._tag === "Some"
					? envLogLevel.value
					: yield* References.MinimumLogLevel;

		return yield* Effect.sync(() => {
			configureDefaultLogger(toPinoLevel(logLevel));
			return getBaseLogger();
		});
	},
);

export const getOrCreateBaseLogger: Effect.Effect<PinoLogger> = Effect.gen(
	function* () {
		const provided = yield* Effect.serviceOption(BaseLogger);
		if (provided._tag === "Some") {
			return provided.value;
		}

		return yield* makeDefaultBaseLogger;
	},
);

export function makeActorLogAnnotations(context: ActorLogContext): {
	readonly actor: string;
	readonly key: string;
	readonly actorId: string;
} {
	return {
		actor: context.name,
		key: serializeActorKey(context.key),
		actorId: context.actorId,
	};
}

export function serializeActorKey(key: Rivetkit.ActorKey): string {
	if (key.length === 0) {
		return EMPTY_KEY;
	}

	return key
		.map((part) => {
			if (part === "") {
				return "\\0";
			}

			return part
				.replace(/\\/g, "\\\\")
				.replace(/\//g, `\\${KEY_SEPARATOR}`);
		})
		.join(KEY_SEPARATOR);
}

function structuredValue(value: unknown): unknown {
	if (value instanceof Error) {
		return value;
	}

	return value;
}

function extractMessageAndFields(message: unknown): {
	readonly msg: string | undefined;
	readonly fields: Record<string, unknown>;
} {
	const values = Array.isArray(message) ? message : [message];
	if (values.length === 0) {
		return { msg: undefined, fields: {} };
	}

	const [first, ...rest] = values;
	const fields: Record<string, unknown> = {};
	let msg: string | undefined;

	if (first instanceof Error) {
		fields.error = first;
		msg = first.message;
	} else if (first !== null && typeof first === "object") {
		const firstFields = first as Record<string, unknown>;
		for (const [key, value] of Object.entries(firstFields)) {
			if (key === "msg") {
				if (value !== undefined) {
					msg = String(value);
				}
			} else {
				fields[key] = structuredValue(value);
			}
		}
	} else if (first !== undefined) {
		msg = String(first);
	}

	const args: Array<unknown> = [];
	for (const value of rest) {
		if (value instanceof Error) {
			fields.error = value;
		} else if (
			value !== null &&
			typeof value === "object" &&
			!Array.isArray(value)
		) {
			for (const [key, fieldValue] of Object.entries(
				value as Record<string, unknown>,
			)) {
				fields[key] = structuredValue(fieldValue);
			}
		} else {
			args.push(value);
		}
	}

	if (args.length > 0) {
		fields.args = args;
	}

	return { msg, fields };
}

export function makeEffectLogger(
	baseLogger: PinoLogger,
): EffectLogger.Logger<unknown, void> {
	return EffectLogger.make(({ cause, date, fiber, logLevel, message }) => {
		const { msg, fields } = extractMessageAndFields(message);

		for (const [key, value] of Object.entries(
			fiber.getRef(References.CurrentLogAnnotations),
		)) {
			fields[key] = structuredValue(value);
		}

		const spans: Record<string, number> = {};
		for (const [label, startTime] of fiber.getRef(
			References.CurrentLogSpans,
		)) {
			spans[label] = date.getTime() - startTime;
		}
		if (Object.keys(spans).length > 0) {
			fields.spans = spans;
		}

		if (cause.reasons.length > 0) {
			fields.cause = Cause.pretty(cause);
		}

		const pinoLevel = toPinoLevel(logLevel);
		if (pinoLevel === "silent") {
			return;
		}

		const logger = baseLogger[pinoLevel];
		if (msg === undefined) {
			logger.call(baseLogger, fields);
		} else {
			logger.call(baseLogger, fields, msg);
		}
	});
}
