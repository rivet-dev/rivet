import {
	Config,
	Context,
	Effect,
	Logger,
	Option,
	Predicate,
	Record as EffectRecord,
	type LogLevel,
	References,
} from "effect";
import type * as Rivetkit from "rivetkit";
import * as RivetkitLog from "rivetkit/log";

const EMPTY_KEY = "/";
const KEY_SEPARATOR = "/";

type ActorLogContext = {
	readonly name: string;
	readonly key: Rivetkit.ActorKey;
	readonly actorId: string;
};

export class BaseLogger extends Context.Service<
	BaseLogger,
	RivetkitLog.Logger
>()("@rivetkit/effect/RivetLogger/BaseLogger") {}

const RivetkitLogLevels = RivetkitLog.LogLevelSchema.options;

const EffectLevelByRivetkitLevel = {
	trace: "Trace",
	debug: "Debug",
	info: "Info",
	warn: "Warn",
	error: "Error",
	fatal: "Fatal",
	silent: "None",
} as const satisfies Record<
	RivetkitLog.LogLevel,
	Exclude<LogLevel.LogLevel, "All">
>;

const RivetkitLevelByEffectLevel = {
	...Object.fromEntries(
		RivetkitLogLevels.map((level) => [
			EffectLevelByRivetkitLevel[level],
			level,
		]),
	),
	All: "trace",
} as Record<LogLevel.LogLevel, RivetkitLog.LogLevel>;

const rivetLogLevelFromEnv = Config.string("RIVET_LOG_LEVEL").pipe(
	Effect.option,
	Effect.map((maybeRivetLogLevel) => {
		if (Option.isNone(maybeRivetLogLevel)) return Option.none();

		const parsed = RivetkitLog.LogLevelSchema.safeParse(
			maybeRivetLogLevel.value.toLowerCase(),
		);
		return parsed.success ? Option.some(parsed.data) : Option.none();
	}),
);

export const makeDefaultBaseLogger: Effect.Effect<RivetkitLog.Logger> =
	Effect.gen(function* () {
		const maybeRivetLogLevel = yield* rivetLogLevelFromEnv;
		const logLevel = Option.isSome(maybeRivetLogLevel)
			? maybeRivetLogLevel.value
			: RivetkitLevelByEffectLevel[yield* References.MinimumLogLevel];

		return yield* Effect.sync(() =>
			RivetkitLog.makeDefaultLogger(logLevel),
		);
	});

export const getOrCreateBaseLogger: Effect.Effect<RivetkitLog.Logger> =
	Effect.gen(function* () {
		const maybeBaseLogger = yield* Effect.serviceOption(BaseLogger);
		if (Option.isSome(maybeBaseLogger)) {
			return maybeBaseLogger.value;
		}

		return yield* makeDefaultBaseLogger;
	});

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

export function makeLogger(
	baseLogger: RivetkitLog.Logger,
): Logger.Logger<unknown, void> {
	return Logger.make((options) => {
		if (options.logLevel === "None") return;
		const rivetkitLevel = RivetkitLevelByEffectLevel[options.logLevel];
		const structured = Logger.formatStructured.log(options);
		const { msg, fields: messageFields } = extractMessage(
			structured.message,
		);
		const fields: Record<string, unknown> = {
			...messageFields,
			...structured.annotations,
			fiberId: structured.fiberId,
		};

		if (!EffectRecord.isEmptyRecord(structured.spans)) {
			fields.spans = structured.spans;
		}
		if (structured.cause !== undefined) {
			fields.cause = structured.cause;
		}

		const logger = baseLogger[rivetkitLevel];
		if (msg === undefined) {
			logger.call(baseLogger, fields);
		} else {
			logger.call(baseLogger, fields, msg);
		}
	});
}

function extractMessage(message: unknown): {
	readonly msg: string | undefined;
	readonly fields: Record<string, unknown>;
} {
	const values = Array.isArray(message) ? message : [message];
	const fields: Record<string, unknown> = {};
	const args: Array<unknown> = [];
	let msg: string | undefined;

	for (const [index, value] of values.entries()) {
		if (Predicate.isError(value)) {
			fields.error = value;
			if (index === 0) msg = value.message;
		} else if (isStructuredError(value)) {
			fields.error = value;
			if (index === 0) msg = value.error;
		} else if (Predicate.isObject(value)) {
			if (index === 0) {
				const { msg: valueMsg, ...rest } = value;
				Object.assign(fields, rest);
				if (valueMsg !== undefined) msg = String(valueMsg);
			} else {
				Object.assign(fields, value);
			}
		} else if (index === 0) {
			msg = value === undefined ? undefined : String(value);
		} else {
			args.push(value);
		}
	}

	if (args.length > 0) {
		fields.args = args;
	}

	return { msg, fields };
}

function isStructuredError(
	value: unknown,
): value is { readonly error: string; readonly name: string } {
	return (
		Predicate.hasProperty(value, "error") &&
		Predicate.hasProperty(value, "name") &&
		Predicate.isString(value.error) &&
		Predicate.isString(value.name)
	);
}
