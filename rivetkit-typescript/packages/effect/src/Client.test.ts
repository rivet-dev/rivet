import { assert, describe, it } from "@effect/vitest";
import { Effect, Schema } from "effect";
import * as RivetkitErrors from "rivetkit/errors";
import * as Client from "./Client";
import * as ActionError from "./internal/ActionError";
import * as RivetError from "./RivetError";

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
							_tag: ActionError.ActionErrorMetadataTag,
							version: ActionError.ActionErrorSchemaVersion,
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
							_tag: ActionError.ActionErrorMetadataTag,
							version: ActionError.ActionErrorSchemaVersion,
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
