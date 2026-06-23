import { assert, describe, it } from "@effect/vitest";
import { ConfigProvider, Effect, Logger, References } from "effect";
import * as RivetkitLog from "rivetkit/log";
import * as Logging from "./logging.ts";

type LogEntry = {
	readonly level: string;
	readonly fields: Record<string, unknown>;
	readonly msg: string | undefined;
};

function makeTestLogger(entries: Array<LogEntry>): RivetkitLog.Logger {
	const logger: Record<string, unknown> = {};
	for (const level of ["trace", "debug", "info", "warn", "error", "fatal"]) {
		logger[level] = (
			fields: Record<string, unknown>,
			msg?: string,
		): void => {
			entries.push({ level, fields, msg });
		};
	}

	return logger as unknown as RivetkitLog.Logger;
}

describe("internal/logging", () => {
	it("serializes actor keys like the RivetKit actor runtime logger", () => {
		assert.strictEqual(Logging.serializeActorKey([]), "/");
		assert.strictEqual(Logging.serializeActorKey(["room", "1"]), "room/1");
		assert.strictEqual(Logging.serializeActorKey(["room/1"]), "room\\/1");
		assert.strictEqual(Logging.serializeActorKey([""]), "\\0");
		assert.strictEqual(Logging.serializeActorKey(["a\\b"]), "a\\\\b");
	});

	it("builds actor log annotations with serialized keys", () => {
		assert.deepStrictEqual(
			Logging.makeActorLogAnnotations({
				name: "ChatRoom",
				key: ["room/1"],
				actorId: "actor-1",
			}),
			{
				actor: "ChatRoom",
				key: "room\\/1",
				actorId: "actor-1",
			},
		);
	});

	it.effect("writes Effect logs through the RivetKit base logger", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);

			yield* Effect.logInfo("room awake", { roomId: "abc" }).pipe(
				Effect.annotateLogs({
					actor: "ChatRoom",
					key: "room-1",
					actorId: "actor-1",
				}),
				Effect.provide(Logger.layer([Logging.makeLogger(baseLogger)])),
			);

			const entry = entries[0];
			assert.ok(entry !== undefined);
			assert.strictEqual(entry.level, "info");
			assert.strictEqual(entry.msg, "room awake");
			assert.deepStrictEqual(entry.fields, {
				roomId: "abc",
				actor: "ChatRoom",
				key: "room-1",
				actorId: "actor-1",
				fiberId: entry.fields.fiberId,
			});
			assert.strictEqual(typeof entry.fields.fiberId, "string");
		}),
	);

	it.effect("preserves Error log messages as structured error fields", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);
			const error = new Error("room failed to wake");

			yield* Effect.logError(error).pipe(
				Effect.provide(Logger.layer([Logging.makeLogger(baseLogger)])),
			);

			const entry = entries[0];
			assert.ok(entry !== undefined);
			assert.strictEqual(entry.level, "error");
			assert.strictEqual(entry.fields.error, error);
			assert.strictEqual(entry.msg, error.message);
		}),
	);

	it.effect("preserves Error log messages with additional fields", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);
			const error = new Error("action dispatch failed");

			yield* Effect.logError(error, {
				actorId: "actor-1",
				action: "SendMessage",
			}).pipe(
				Effect.provide(Logger.layer([Logging.makeLogger(baseLogger)])),
			);

			const entry = entries[0];
			assert.ok(entry !== undefined);
			assert.strictEqual(entry.level, "error");
			assert.strictEqual(entry.fields.error, error);
			assert.strictEqual(entry.fields.actorId, "actor-1");
			assert.strictEqual(entry.fields.action, "SendMessage");
			assert.strictEqual(entry.msg, error.message);
		}),
	);

	it.effect("accepts RIVET_LOG_LEVEL values", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "silent");
		}).pipe(
			Effect.provideService(
				ConfigProvider.ConfigProvider,
				ConfigProvider.fromEnv({
					env: {
						RIVET_LOG_LEVEL: "silent",
					},
				}),
			),
		),
	);

	it.effect("accepts uppercase RIVET_LOG_LEVEL values", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "debug");
		}).pipe(
			Effect.provideService(
				ConfigProvider.ConfigProvider,
				ConfigProvider.fromEnv({
					env: {
						RIVET_LOG_LEVEL: "DEBUG",
					},
				}),
			),
		),
	);

	it.effect("ignores Effect-only RIVET_LOG_LEVEL values", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "info");
		}).pipe(
			Effect.provideService(References.MinimumLogLevel, "Info"),
			Effect.provideService(
				ConfigProvider.ConfigProvider,
				ConfigProvider.fromEnv({
					env: {
						RIVET_LOG_LEVEL: "None",
					},
				}),
			),
		),
	);

	it.effect("falls back to References.MinimumLogLevel without env", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "debug");
		}).pipe(
			Effect.provideService(References.MinimumLogLevel, "Debug"),
			Effect.provideService(
				ConfigProvider.ConfigProvider,
				ConfigProvider.fromEnv({
					env: {},
				}),
			),
		),
	);

	it.effect("RIVET_LOG_LEVEL overrides References.MinimumLogLevel", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "silent");
		}).pipe(
			Effect.provideService(References.MinimumLogLevel, "Debug"),
			Effect.provideService(
				ConfigProvider.ConfigProvider,
				ConfigProvider.fromEnv({
					env: {
						RIVET_LOG_LEVEL: "silent",
					},
				}),
			),
		),
	);

	it.effect(
		"uses References.CurrentLogLevel for plain Effect.log calls",
		() =>
			Effect.gen(function* () {
				const entries: Array<LogEntry> = [];
				const baseLogger = makeTestLogger(entries);

				yield* Effect.log("plain log").pipe(
					Effect.provideService(References.CurrentLogLevel, "Debug"),
					Effect.provideService(References.MinimumLogLevel, "Debug"),
					Effect.provide(
						Logger.layer([Logging.makeLogger(baseLogger)]),
					),
				);

				const entry = entries[0];
				assert.ok(entry !== undefined);
				assert.strictEqual(entry.level, "debug");
				assert.strictEqual(entry.msg, "plain log");
				assert.deepStrictEqual(entry.fields, {
					fiberId: entry.fields.fiberId,
				});
				assert.strictEqual(typeof entry.fields.fiberId, "string");
			}),
	);

	it.effect(
		"does not call a Pino method for the None current log level",
		() =>
			Effect.gen(function* () {
				const entries: Array<LogEntry> = [];
				const baseLogger = makeTestLogger(entries);

				yield* Effect.log("hidden log").pipe(
					Effect.provideService(References.CurrentLogLevel, "None"),
					Effect.provideService(References.MinimumLogLevel, "All"),
					Effect.provide(
						Logger.layer([Logging.makeLogger(baseLogger)]),
					),
				);

				assert.deepStrictEqual(entries, []);
			}),
	);

	it.effect(
		"emits References.CurrentLogSpans as structured span durations",
		() =>
			Effect.gen(function* () {
				const entries: Array<LogEntry> = [];
				const baseLogger = makeTestLogger(entries);

				yield* Effect.logInfo("checkout complete").pipe(
					Effect.withLogSpan("checkout"),
					Effect.provide(
						Logger.layer([Logging.makeLogger(baseLogger)]),
					),
				);

				assert.strictEqual(entries.length, 1);
				assert.strictEqual(entries[0]?.level, "info");
				assert.strictEqual(entries[0]?.msg, "checkout complete");
				const spans = entries[0]?.fields.spans as
					| Record<string, unknown>
					| undefined;
				assert.strictEqual(typeof spans?.checkout, "number");
			}),
	);
});
