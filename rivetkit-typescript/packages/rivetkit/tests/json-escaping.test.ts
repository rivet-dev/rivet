import { describe, expect, test } from "vitest";
import { jsonParseCompat, jsonStringifyCompat } from "@/actor/protocol/serde";

describe("JSON Escaping", () => {
	describe("BigInt", () => {
		test("should serialize and deserialize BigInt", () => {
			const input = { num: BigInt(123) };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.num).toBe(BigInt(123));
		});

		test("should handle negative BigInt", () => {
			const input = { num: BigInt(-456) };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.num).toBe(BigInt(-456));
		});

		test("should handle very large BigInt", () => {
			const input = { num: BigInt("9007199254740991999") };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.num).toBe(BigInt("9007199254740991999"));
		});
	});

	describe("ArrayBuffer", () => {
		test("should serialize and deserialize ArrayBuffer", () => {
			const buffer = new ArrayBuffer(4);
			const view = new Uint8Array(buffer);
			view[0] = 1;
			view[1] = 2;
			view[2] = 3;
			view[3] = 4;

			const input = { buf: buffer };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			const resultView = new Uint8Array(result.buf);
			expect(resultView[0]).toBe(1);
			expect(resultView[1]).toBe(2);
			expect(resultView[2]).toBe(3);
			expect(resultView[3]).toBe(4);
		});
	});

	describe("Uint8Array", () => {
		test("should serialize and deserialize Uint8Array", () => {
			const input = { arr: new Uint8Array([1, 2, 3, 4, 5]) };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.arr).toBeInstanceOf(Uint8Array);
			expect(result.arr[0]).toBe(1);
			expect(result.arr[1]).toBe(2);
			expect(result.arr[2]).toBe(3);
			expect(result.arr[3]).toBe(4);
			expect(result.arr[4]).toBe(5);
		});
	});

	describe("User data with $-prefixed properties", () => {
		test("should preserve user $type property", () => {
			const input = { $type: "MyCustomType", value: 456 };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			// User properties are preserved as-is
			expect(result.$type).toBe("MyCustomType");
			expect(result.value).toBe(456);
		});

		test("should preserve user $$type property", () => {
			const input = { $$type: "NestedType", id: 1 };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.$$type).toBe("NestedType");
			expect(result.id).toBe(1);
		});

		test("should escape user arrays starting with $", () => {
			const input = {
				userArray: ["$NotAType", "someValue"],
				userType: { $type: "UserType1", id: 1 },
			};
			const json = jsonStringifyCompat(input);

			// User array should be escaped to $$NotAType
			expect(json).toContain('"$$NotAType"');

			const result = jsonParseCompat(json);

			// Should be unescaped back to original
			expect(result.userArray).toEqual(["$NotAType", "someValue"]);
			expect(result.userType.$type).toBe("UserType1");
		});

		test("should handle user arrays already starting with $$", () => {
			const input = { arr: ["$$AlreadyEscaped", "value"] };
			const json = jsonStringifyCompat(input);

			// Should add another $ to escape
			expect(json).toContain('"$$$AlreadyEscaped"');

			const result = jsonParseCompat(json);

			// Should be unescaped back to original
			expect(result.arr).toEqual(["$$AlreadyEscaped", "value"]);
		});
	});

	describe("Complex scenarios", () => {
		test("should handle mix of special types and user properties", () => {
			const input = {
				bigNum: BigInt(999),
				buffer: new Uint8Array([1, 2, 3]),
				metadata: { $type: "UserType", id: 1 },
			};
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.bigNum).toBe(BigInt(999));
			expect(result.buffer).toBeInstanceOf(Uint8Array);
			expect(result.buffer[0]).toBe(1);
			expect(result.metadata.$type).toBe("UserType");
		});

		test("should handle deeply nested structures", () => {
			const input = {
				level1: {
					level2: {
						level3: {
							bigNum: BigInt(123),
							metadata: { $type: "Deep", value: 456 },
						},
					},
				},
			};
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.level1.level2.level3.bigNum).toBe(BigInt(123));
			expect(result.level1.level2.level3.metadata.$type).toBe("Deep");
			expect(result.level1.level2.level3.metadata.value).toBe(456);
		});

		test("should handle arrays containing special types", () => {
			const input = {
				items: [
					{ num: BigInt(1) },
					{ metadata: { $type: "ArrayItem", index: 0 } },
					{ arr: new Uint8Array([255]) },
				],
			};
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.items[0].num).toBe(BigInt(1));
			expect(result.items[1].metadata.$type).toBe("ArrayItem");
			expect(result.items[2].arr).toBeInstanceOf(Uint8Array);
			expect(result.items[2].arr[0]).toBe(255);
		});
	});

	describe("Edge cases", () => {
		test("should handle empty objects", () => {
			const input = {};
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result).toEqual({});
		});

		test("should handle null values", () => {
			const input = { value: null };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			expect(result.value).toBeNull();
		});

		test("should handle undefined values", () => {
			const input = { value: undefined, other: 123 };
			const json = jsonStringifyCompat(input);
			const result = jsonParseCompat(json);

			// undefined gets dropped by JSON.stringify
			expect(result.value).toBeUndefined();
			expect(result.other).toBe(123);
		});

		test("should escape and unescape user arrays with unknown $ types", () => {
			const input = { arr: ["$CustomType", "custom value"] };
			const json = jsonStringifyCompat(input);

			// Should be escaped to $$CustomType
			expect(json).toContain('"$$CustomType"');

			const result = jsonParseCompat(json);

			// Should be unescaped back to original
			expect(result.arr).toEqual(["$CustomType", "custom value"]);
			expect(Array.isArray(result.arr)).toBe(true);
		});

		test("should throw error on unrecognized type starting with single $", () => {
			// Manually construct invalid JSON (this shouldn't happen if encoding worked)
			const invalidJson = '{"arr":["$UnknownType","value"]}';

			expect(() => jsonParseCompat(invalidJson)).toThrow(
				"Unknown JSON encoding type: $UnknownType",
			);
		});
	});
});
