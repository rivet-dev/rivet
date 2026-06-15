import { assert, describe, it } from "@effect/vitest";
import { Client, Logger, RivetError } from "@rivetkit/effect";
import { Effect, Layer, Schema } from "effect";
import * as RivetkitErrors from "rivetkit/errors";
import {
	configureDefaultLogger,
	getBaseLogger,
	type Logger as PinoLogger,
} from "rivetkit/log";
import * as ActionErrorEnvelope from "./internal/ActionErrorEnvelope";

function makeTestLogger(
	entries?: Array<{
		readonly level: string;
		readonly fields: Record<string, unknown>;
		readonly msg: string | undefined;
	}>,
): PinoLogger {
	const logger: Record<string, unknown> = {
		level: "debug",
		child: () => logger,
	};
	for (const level of ["trace", "debug", "info", "warn", "error", "fatal"]) {
		logger[level] = (
			fields: Record<string, unknown>,
			msg?: string,
		): void => {
			entries?.push({ level, fields, msg });
		};
	}

	return logger as unknown as PinoLogger;
}

describe("Client", () => {
	it.effect("configures the underlying RivetKit client logger", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const baseLogger = makeTestLogger();

				yield* Effect.addFinalizer(() =>
					Effect.sync(() => configureDefaultLogger("silent")),
				);
				yield* Client.make({
					endpoint: "http://127.0.0.1:6420",
				}).pipe(Effect.provide(Logger.layerPino(baseLogger)));

				assert.strictEqual(getBaseLogger(), baseLogger);
			}),
		),
	);

	it.effect("installs the RivetKit Effect logger for client programs", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const entries: Array<{
					readonly level: string;
					readonly fields: Record<string, unknown>;
					readonly msg: string | undefined;
				}> = [];
				const baseLogger = makeTestLogger(entries);

				yield* Effect.addFinalizer(() =>
					Effect.sync(() => configureDefaultLogger("silent")),
				);
				yield* Effect.gen(function* () {
					yield* Client.Client;
					yield* Effect.logInfo("client effect log", {
						clientId: "test-client",
					});
				}).pipe(
					Effect.provide(
						Client.layer({
							endpoint: "http://127.0.0.1:6420",
						}).pipe(
							Layer.provideMerge(Logger.layerPino(baseLogger)),
						),
					),
				);

				assert.deepStrictEqual(entries[0], {
					level: "info",
					fields: { clientId: "test-client" },
					msg: "client effect log",
				});
				assert.ok(
					entries.some(
						(entry) =>
							entry.level === "debug" &&
							(entry.fields as { msg?: unknown }).msg ===
								"disposing client",
					),
				);
			}),
		),
	);
});

describe("makeRivetkitActionFailureClassifier", () => {
	const ExpectedError = Schema.Struct({
		_tag: Schema.tag("CounterOverflow"),
		message: Schema.String,
		limit: Schema.Number,
	});
	const classifyRivetkitActionFailure =
		Client.makeRivetkitActionFailureClassifier(ExpectedError);

	it.effect("preserves non-Rivet failures as UnknownError", () =>
		Effect.gen(function* () {
			const cause = new Error("plain failure");
			const error = yield* classifyRivetkitActionFailure(cause);

			assert.instanceOf(error, RivetError.RivetError);
			assert.instanceOf(error.reason, RivetError.UnknownError);
			assert.strictEqual(error.reason.message, "plain failure");
			assert.strictEqual(error.reason.cause, cause);
		}),
	);

	it.effect("preserves structured non-action Rivet errors", () =>
		Effect.gen(function* () {
			const cause = new RivetkitErrors.RivetError(
				"actor",
				"not_found",
				"actor not found",
			);
			const error = yield* classifyRivetkitActionFailure(cause);

			assert.instanceOf(error, RivetError.RivetError);
			assert.instanceOf(error.reason, RivetError.ActorNotFound);
			assert.strictEqual(error.reason.cause.group, "actor");
			assert.strictEqual(error.reason.cause.code, "not_found");
			assert.strictEqual(error.reason.cause.message, "actor not found");
		}),
	);

	it.effect(
		"decodes action-error metadata into the declared error type",
		() =>
			Effect.gen(function* () {
				const cause = new RivetkitErrors.RivetError(
					"user",
					"CounterOverflow",
					"counter overflow",
					{
						public: true,
						metadata: {
							_tag: ActionErrorEnvelope.tag,
							version: ActionErrorEnvelope.schemaVersion,
							error: {
								_tag: "CounterOverflow",
								message: "counter overflow",
								limit: 10,
							},
						},
					},
				);
				const error = yield* classifyRivetkitActionFailure(cause);

				assert.deepStrictEqual(error, {
					_tag: "CounterOverflow",
					message: "counter overflow",
					limit: 10,
				});
			}),
	);

	it.effect(
		"wraps invalid typed action-error payloads in ActionErrorDecodeFailed",
		() =>
			Effect.gen(function* () {
				const cause = new RivetkitErrors.RivetError(
					"user",
					"CounterOverflow",
					"counter overflow",
					{
						metadata: {
							_tag: ActionErrorEnvelope.tag,
							version: ActionErrorEnvelope.schemaVersion,
							error: {
								_tag: "CounterOverflow",
								message: "counter overflow",
								limit: "10",
							},
						},
					},
				);

				const error = yield* classifyRivetkitActionFailure(cause);

				assert.instanceOf(error, RivetError.RivetError);
				assert.instanceOf(
					error.reason,
					RivetError.ActionErrorDecodeFailed,
				);
				assert.strictEqual(error.reason.rivetError.group, "user");
				assert.strictEqual(
					error.reason.rivetError.code,
					"CounterOverflow",
				);
			}),
	);
});
