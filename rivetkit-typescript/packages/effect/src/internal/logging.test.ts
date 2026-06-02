import { assert, describe, it } from "@effect/vitest";
import {
	Config,
	ConfigProvider,
	Effect,
	Layer,
	Logger as EffectLogger,
	References,
} from "effect";
import type { Logger as PinoLogger } from "rivetkit/log";
import * as Logging from "./logging.ts";

type LogEntry = {
	readonly level: string;
	readonly fields: Record<string, unknown>;
	readonly msg: string | undefined;
};

function makeTestLogger(entries: Array<LogEntry>): PinoLogger {
	const logger: Record<string, unknown> = {};
	for (const level of [
		"trace",
		"debug",
		"info",
		"warn",
		"error",
		"fatal",
	]) {
		logger[level] = (
			fields: Record<string, unknown>,
			msg?: string,
		): void => {
			entries.push({ level, fields, msg });
		};
	}

	return logger as unknown as PinoLogger;
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
				Effect.provide(
					EffectLogger.layer([Logging.makeEffectLogger(baseLogger)]),
				),
			);

			assert.deepStrictEqual(entries, [
				{
					level: "info",
					fields: {
						roomId: "abc",
						actor: "ChatRoom",
						key: "room-1",
						actorId: "actor-1",
					},
					msg: "room awake",
				},
			]);
		}),
	);

	it.effect("preserves Error log messages as structured error fields", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);
			const error = new Error("room failed to wake");

			yield* Effect.logError(error).pipe(
				Effect.provide(
					EffectLogger.layer([Logging.makeEffectLogger(baseLogger)]),
				),
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
				Effect.provide(
					EffectLogger.layer([Logging.makeEffectLogger(baseLogger)]),
				),
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

	it.effect("uses References.MinimumLogLevel when creating the base logger", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "debug");
		}).pipe(Effect.provideService(References.MinimumLogLevel, "Debug")),
	);

	it.effect("accepts the shared Pino RIVET_LOG_LEVEL values", () =>
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

	it.effect("prefers References.MinimumLogLevel over shared env values", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "debug");
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

	it.effect("preserves an explicit Info minimum log level", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "info");
		}).pipe(
			Effect.provideService(References.MinimumLogLevel, "Info"),
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

	it.effect("uses Config.logLevel values provided to References.MinimumLogLevel", () =>
		Effect.gen(function* () {
			const baseLogger = yield* Logging.makeDefaultBaseLogger;

			assert.strictEqual(baseLogger.level, "trace");
		}).pipe(
			Effect.provide(
				Layer.effect(
					References.MinimumLogLevel,
					Config.logLevel("RIVET_LOG_LEVEL"),
				),
			),
			Effect.provideService(
				ConfigProvider.ConfigProvider,
				ConfigProvider.fromEnv({
					env: {
						RIVET_LOG_LEVEL: "Trace",
					},
				}),
			),
		),
	);

	it.effect("uses References.CurrentLogLevel for plain Effect.log calls", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);

			yield* Effect.log("plain log").pipe(
				Effect.provideService(References.CurrentLogLevel, "Debug"),
				Effect.provideService(References.MinimumLogLevel, "Debug"),
				Effect.provide(
					EffectLogger.layer([Logging.makeEffectLogger(baseLogger)]),
				),
			);

			assert.deepStrictEqual(entries, [
				{
					level: "debug",
					fields: {},
					msg: "plain log",
				},
			]);
		}),
	);

	it.effect("does not call a Pino method for the None current log level", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);

			yield* Effect.log("hidden log").pipe(
				Effect.provideService(References.CurrentLogLevel, "None"),
				Effect.provideService(References.MinimumLogLevel, "All"),
				Effect.provide(
					EffectLogger.layer([Logging.makeEffectLogger(baseLogger)]),
				),
			);

			assert.deepStrictEqual(entries, []);
		}),
	);

	it.effect("emits References.CurrentLogSpans as structured span durations", () =>
		Effect.gen(function* () {
			const entries: Array<LogEntry> = [];
			const baseLogger = makeTestLogger(entries);

			yield* Effect.logInfo("checkout complete").pipe(
				Effect.withLogSpan("checkout"),
				Effect.provide(
					EffectLogger.layer([Logging.makeEffectLogger(baseLogger)]),
				),
			);

			assert.strictEqual(entries.length, 1);
			assert.strictEqual(entries[0]?.level, "info");
			assert.strictEqual(entries[0]?.msg, "checkout complete");
			assert.deepStrictEqual(Object.keys(entries[0]?.fields ?? {}), [
				"spans",
			]);
			const spans = entries[0]?.fields.spans as
				| Record<string, unknown>
				| undefined;
			assert.strictEqual(typeof spans?.checkout, "number");
		}),
	);
});
