import { describe, expect, test } from "vitest";
import {
	RivetError,
	decodeBridgeRivetError,
	encodeBridgeRivetError,
	toRivetError,
} from "../src/actor/errors";
import { deconstructError } from "../src/common/utils";

describe("RivetError bridge helpers", () => {
	test("round trips structured bridge payloads", () => {
		const error = new RivetError("user", "boom", "typed failure", {
			metadata: { source: "native" },
			public: true,
			actor: {
				actorId: "actor-123",
				generation: 7,
				key: "chat/1",
			},
		});

		const decoded = decodeBridgeRivetError(encodeBridgeRivetError(error));

		expect(decoded).toBeInstanceOf(RivetError);
		expect(decoded).toMatchObject({
			group: "user",
			code: "boom",
			message: "typed failure",
			metadata: { source: "native" },
			actor: {
				actorId: "actor-123",
				generation: 7,
				key: "chat/1",
			},
		});
	});

	test("wraps plain errors with actor/internal_error defaults", () => {
		const error = toRivetError(new Error("plain failure"), {
			group: "actor",
			code: "internal_error",
		});

		expect(error).toMatchObject({
			group: "actor",
			code: "internal_error",
			message: "plain failure",
		});
	});

	test("passes through canonical RivetError instances", () => {
		const error = new RivetError(
			"actor",
			"action_timed_out",
			"Action timed out",
			{
				public: true,
				statusCode: 408,
				metadata: { source: "core" },
			},
		);

		const result = deconstructError(error);

		expect(result).toMatchObject({
			statusCode: 408,
			public: true,
			group: "actor",
			code: "action_timed_out",
			message: "Action timed out",
			metadata: { source: "core" },
		});
	});

	test("does not treat plain objects as structured errors", () => {
		const result = deconstructError(
			{ group: "foo", code: "bar", message: "baz" },
		);

		expect(result).toMatchObject({
			statusCode: 500,
			public: false,
			group: "rivetkit",
			code: "internal_error",
			message: "An internal error occurred",
		});
	});

	test("classifies malformed tagged RivetError payloads", () => {
		const result = deconstructError(
			{ __type: "RivetError", code: "bar", message: "baz" },
			true,
		);

		expect(result).toMatchObject({
			statusCode: 500,
			public: false,
			group: "rivetkit",
			code: "internal_error",
			message: "baz",
		});
	});
});
