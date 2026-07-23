import { describe, expect, test } from "vitest";
import { HttpQueueStatusResponseSchema } from "../src/common/client-protocol-zod";

describe("client protocol schemas", () => {
	test("normalizes CBOR queue receipt integers and null options", () => {
		expect(
			HttpQueueStatusResponseSchema.parse({
				state: "queued",
				attempts: 2n,
				createdAt: 42n,
				availableAt: null,
				startedAt: null,
				completedAt: null,
				failedAt: null,
				consumedAt: null,
			}),
		).toEqual({
			state: "queued",
			attempts: 2,
			createdAt: 42,
		});
	});

	test("rejects queue receipt integers that are not safely representable", () => {
		expect(() =>
			HttpQueueStatusResponseSchema.parse({
				state: "queued",
				createdAt: BigInt(Number.MAX_SAFE_INTEGER) + 1n,
			}),
		).toThrow();
	});

	test("rejects queue receipt states missing their required timestamp", () => {
		expect(() =>
			HttpQueueStatusResponseSchema.parse({
				state: "succeeded",
				attempts: 1,
				createdAt: 42,
			}),
		).toThrow();
	});
});
