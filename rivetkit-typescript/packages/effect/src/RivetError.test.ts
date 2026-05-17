import { assert, describe, it } from "@effect/vitest";
import * as RivetkitErrors from "rivetkit/errors";
import * as RivetError from "./RivetError";

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
			tunnelTimeout.reason,
			RivetError.GuardTunnelMessageTimeout,
		);
		assert.strictEqual(serviceUnavailable.reason.code, "service_unavailable");
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
});
