import { describe, expect, test } from "vitest";
import {
	normalizeRuntimeSqlExecuteResult,
	type RuntimeSqlBindParam,
	type RuntimeSqlBindParams,
} from "./runtime";

describe("runtime SQL boundary", () => {
	test("accepts exact bind param variants", () => {
		const blob = new Uint8Array([1, 2, 3]);
		const params = [
			{ kind: "null" },
			{ kind: "int", intValue: 1 },
			{ kind: "float", floatValue: 1.5 },
			{ kind: "text", textValue: "text" },
			{ kind: "blob", blobValue: blob },
		] satisfies RuntimeSqlBindParams;

		expect(params).toEqual([
			{ kind: "null" },
			{ kind: "int", intValue: 1 },
			{ kind: "float", floatValue: 1.5 },
			{ kind: "text", textValue: "text" },
			{ kind: "blob", blobValue: blob },
		]);
	});

	test("rejects bind params with mismatched value fields at typecheck time", () => {
		const invalidIntParamCandidate = {
			kind: "int",
			intValue: 1,
			textValue: "extra",
		} as const;
		// @ts-expect-error Runtime SQL int params must only carry intValue.
		const invalidIntParam: RuntimeSqlBindParam = invalidIntParamCandidate;

		expect(invalidIntParam.kind).toBe("int");
	});

	test("normalizes execute result metadata", () => {
		const base = {
			columns: ["value"],
			rows: [[1]],
			changes: 1,
			lastInsertRowId: null,
		};

		expect(normalizeRuntimeSqlExecuteResult(base)).toEqual(base);
	});
});
