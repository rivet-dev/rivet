import { describe, expect, test } from "vitest";
import { z } from "zod/v4";
import { event, queue } from "../src/actor/schema";
import {
	validateActionArgs,
	validateConnParams,
	validateEventArgs,
	validateQueueBody,
} from "../src/registry/native-validation";

describe("native validation helpers", () => {
	test("validateActionArgs returns validated tuples", () => {
		expect(
			validateActionArgs(
				{
					increment: z.tuple([z.object({ amount: z.number().int() })]),
				},
				"increment",
				[{ amount: 2 }],
			),
		).toEqual([{ amount: 2 }]);
	});

	test("validateActionArgs throws RivetError for invalid tuples", () => {
		expectValidationError(() =>
			validateActionArgs(
				{
					increment: z.tuple([z.object({ amount: z.number().int() })]),
				},
				"increment",
				[{ amount: "bad" }],
			),
		);
	});

	test("validateConnParams enforces the configured schema", () => {
		expect(
			validateConnParams(
				z.object({ userId: z.string().min(1) }),
				{ userId: "abc" },
			),
		).toEqual({ userId: "abc" });
		expectValidationError(() =>
			validateConnParams(
				z.object({ userId: z.string().min(1) }),
				{ userId: 42 },
			),
		);
	});

	test("validateEventArgs validates payloads against event schemas", () => {
		expect(
			validateEventArgs(
				{
					countChanged: event({
						schema: z.object({ count: z.number().int() }),
					}),
				},
				"countChanged",
				[{ count: 2 }],
			),
		).toEqual([{ count: 2 }]);
	});

	test("validateQueueBody validates payloads against queue schemas", () => {
		expect(
			validateQueueBody(
				{
					jobs: queue({
						message: z.object({ id: z.string().min(1) }),
					}),
				},
				"jobs",
				{ id: "job-1" },
			),
		).toEqual({ id: "job-1" });
		expectValidationError(() =>
			validateQueueBody(
				{
					jobs: queue({
						message: z.object({ id: z.string().min(1) }),
					}),
				},
				"jobs",
				{ id: "" },
			),
		);
	});
});

function expectValidationError(run: () => unknown) {
	try {
		run();
		throw new Error("expected validation error");
	} catch (error) {
		expect(error).toMatchObject({
			group: "actor",
			code: "validation_error",
		});
	}
}
