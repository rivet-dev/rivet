import { assert, describe, it } from "@effect/vitest";
import { RivetError } from "@rivetkit/effect";
import { Duration, Effect, Schema } from "effect";
import * as RivetkitErrors from "rivetkit/errors";

describe("RivetError", () => {
	it("preserves non-Rivet causes as UnknownError", () => {
		const cause = new Error("plain failure");
		const error = RivetError.fromUnknown(cause);

		assert.instanceOf(error, RivetError.RivetError);
		assert.instanceOf(error.reason, RivetError.UnknownError);
		assert.strictEqual(error.reason.message, "plain failure");
		assert.strictEqual(error.reason.cause, cause);
	});

	it("allows UnknownError to wrap arbitrary causes", () => {
		const cause = { group: "not-a-rivet-error", code: 123 };
		const error = new RivetError.UnknownError({
			message: "malformed failure",
			cause,
		});

		assert.strictEqual(error.cause, cause);
		assert.strictEqual(error.group, undefined);
		assert.strictEqual(error.code, undefined);
	});

	it("keeps structured Rivet errors classified by group and code", () => {
		const cause = new RivetkitErrors.RivetError(
			"rivetkit",
			RivetkitErrors.INTERNAL_ERROR_CODE,
			"internal failure",
		);
		const error = RivetError.fromUnknown(cause);

		assert.instanceOf(error.reason, RivetError.InternalError);
		assert.strictEqual(error.reason.group, cause.group);
		assert.strictEqual(error.reason.code, cause.code);
		assert.strictEqual(error.reason.message, cause.message);
	});

	it("exposes normalized isRetryable on every reason", () => {
		const restarting = RivetError.fromUnknown(
			new RivetkitErrors.RivetError("actor", "restarting", "restarting"),
		);
		const forbidden = RivetError.fromUnknown(
			new RivetkitErrors.RivetError("auth", "forbidden", "forbidden"),
		);
		const overloaded = RivetError.fromUnknown(
			new RivetkitErrors.RivetError("actor", "overloaded", "overloaded"),
		);
		const serviceUnavailable = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"guard",
				"service_unavailable",
				"service unavailable",
			),
		);
		const incomingTooLong = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"message",
				"incoming_too_long",
				"too long",
			),
		);

		assert.strictEqual(restarting.isRetryable, true);
		assert.strictEqual(restarting.reason.isRetryable, true);
		assert.strictEqual(forbidden.isRetryable, false);
		assert.strictEqual(overloaded.isRetryable, true);
		assert.strictEqual(serviceUnavailable.isRetryable, true);
		assert.strictEqual(incomingTooLong.isRetryable, false);
	});

	it("exposes retryAfter from ActorRestarting metadata", () => {
		const restarting = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"actor",
				"restarting",
				"actor restarting",
				{ metadata: { retryAfterMs: 250 } },
			),
		);
		const restartingNoHint = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"actor",
				"restarting",
				"actor restarting",
			),
		);

		assert.instanceOf(restarting.reason, RivetError.ActorRestarting);
		assert.deepStrictEqual(restarting.retryAfter, Duration.millis(250));
		assert.deepStrictEqual(
			restarting.reason.retryAfter,
			Duration.millis(250),
		);
		assert.strictEqual(restartingNoHint.retryAfter, undefined);
	});

	it("returns retryAfter undefined for reasons without retry-timing hints", () => {
		const overloaded = RivetError.fromUnknown(
			new RivetkitErrors.RivetError("actor", "overloaded", "overloaded"),
		);
		assert.strictEqual(overloaded.retryAfter, undefined);
	});

	it("classifies known guard errors into specific reasons", () => {
		const serviceUnavailable = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"guard",
				"service_unavailable",
				"service unavailable",
			),
		);
		const readyTimeout = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"guard",
				"actor_ready_timeout",
				"actor ready timeout",
			),
		);
		const wakeRetriesExceeded = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"guard",
				"actor_wake_retries_exceeded",
				"actor wake retries exceeded",
			),
		);
		const tunnelTimeout = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"guard",
				"tunnel_message_timeout",
				"tunnel message timeout",
			),
		);

		assert.instanceOf(
			serviceUnavailable.reason,
			RivetError.GuardServiceUnavailable,
		);
		assert.instanceOf(
			readyTimeout.reason,
			RivetError.GuardActorReadyTimeout,
		);
		assert.instanceOf(
			wakeRetriesExceeded.reason,
			RivetError.GuardActorWakeRetriesExceeded,
		);
		assert.instanceOf(
			tunnelTimeout.reason,
			RivetError.GuardTunnelMessageTimeout,
		);
		assert.strictEqual(
			serviceUnavailable.reason.code,
			"service_unavailable",
		);
	});

	it("keeps unknown guard errors in UnknownError", () => {
		const error = RivetError.fromUnknown(
			new RivetkitErrors.RivetError(
				"guard",
				"new_guard_code",
				"new guard code",
			),
		);

		assert.instanceOf(error.reason, RivetError.UnknownError);
		assert.strictEqual(error.reason.code, "new_guard_code");
	});

	it("exposes action error decode failures with decode context", () => {
		const cause = new RivetkitErrors.RivetError(
			"user",
			"CounterOverflow",
			"counter overflow",
			{ metadata: { _tag: "EffectActionError", version: 1, error: {} } },
		);
		const schemaError = Effect.runSync(
			Schema.decodeUnknownEffect(Schema.String)(123).pipe(Effect.flip),
		);
		const error = new RivetError.RivetError({
			reason: new RivetError.ActionErrorDecodeFailed({
				cause: schemaError,
				rivetError: cause,
			}),
		});

		assert.instanceOf(error.reason, RivetError.ActionErrorDecodeFailed);
		assert.strictEqual(error.reason.cause, schemaError);
		assert.strictEqual(error.reason.rivetError, cause);
		assert.strictEqual(
			error.reason.message,
			"Failed to decode action error user.CounterOverflow",
		);
	});
});
