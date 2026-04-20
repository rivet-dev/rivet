import { describe, expect, test, vi } from "vitest";
import {
	RivetError,
	decodeBridgeRivetError,
	encodeBridgeRivetError,
	toRivetError,
} from "../src/actor/errors";
import { deconstructError } from "../src/common/utils";

function createLogger() {
	return {
		info: vi.fn(),
		warn: vi.fn(),
	} as any;
}

describe("RivetError bridge helpers", () => {
	test("round trips structured bridge payloads", () => {
		const error = new RivetError("user", "boom", "typed failure", {
			metadata: { source: "native" },
			public: true,
		});

		const decoded = decodeBridgeRivetError(encodeBridgeRivetError(error));

		expect(decoded).toBeInstanceOf(RivetError);
		expect(decoded).toMatchObject({
			group: "user",
			code: "boom",
			message: "typed failure",
			metadata: { source: "native" },
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
		const logger = createLogger();
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

		const result = deconstructError(error, logger, {});

		expect(result).toMatchObject({
			statusCode: 408,
			public: true,
			group: "actor",
			code: "action_timed_out",
			message: "Action timed out",
			metadata: { source: "core" },
		});
		expect(logger.info).toHaveBeenCalledWith(
			expect.objectContaining({
				msg: "structured error passthrough",
				group: "actor",
				code: "action_timed_out",
			}),
		);
	});

	test("does not treat plain objects as structured errors", () => {
		const logger = createLogger();

		const result = deconstructError(
			{ group: "foo", code: "bar", message: "baz" },
			logger,
			{},
		);

		expect(result).toMatchObject({
			statusCode: 500,
			public: false,
			group: "rivetkit",
			code: "internal_error",
			message: "Internal error. Read the server logs for more details.",
		});
		expect(logger.info).not.toHaveBeenCalledWith(
			expect.objectContaining({
				msg: "structured error passthrough",
			}),
		);
	});

	test("classifies malformed tagged RivetError payloads", () => {
		const logger = createLogger();

		const result = deconstructError(
			{ __type: "RivetError", code: "bar", message: "baz" },
			logger,
			{},
			true,
		);

		expect(result).toMatchObject({
			statusCode: 500,
			public: false,
			group: "rivetkit",
			code: "internal_error",
			message: "baz",
		});
		expect(logger.info).not.toHaveBeenCalledWith(
			expect.objectContaining({
				msg: "structured error passthrough",
			}),
		);
	});
});
