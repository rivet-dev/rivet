import { describe, expect, test } from "vitest";
import {
	RivetError,
	decodeBridgeRivetError,
	encodeBridgeRivetError,
	toRivetError,
} from "../src/actor/errors";

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
});
