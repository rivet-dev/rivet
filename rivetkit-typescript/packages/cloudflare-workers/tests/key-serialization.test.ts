// import { describe, expect, test } from "vitest";
// import {
// 	deserializeKey,
// 	EMPTY_KEY,
// 	KEY_SEPARATOR,
// 	serializeKey,
// 	serializeNameAndKey,
// } from "../src/util";
//
// describe("Key serialization and deserialization", () => {
// 	// Test key serialization
// 	describe("serializeKey", () => {
// 		test("serializes empty key array", () => {
// 			expect(serializeKey([])).toBe(EMPTY_KEY);
// 		});
//
// 		test("serializes single key", () => {
// 			expect(serializeKey(["test"])).toBe("test");
// 		});
//
// 		test("serializes multiple keys", () => {
// 			expect(serializeKey(["a", "b", "c"])).toBe(
// 				`a${KEY_SEPARATOR}b${KEY_SEPARATOR}c`,
// 			);
// 		});
//
// 		test("escapes commas in keys", () => {
// 			expect(serializeKey(["a,b"])).toBe("a\\,b");
// 			expect(serializeKey(["a,b", "c"])).toBe(`a\\,b${KEY_SEPARATOR}c`);
// 		});
//
// 		test("escapes empty key marker in keys", () => {
// 			expect(serializeKey([EMPTY_KEY])).toBe(`\\${EMPTY_KEY}`);
// 		});
//
// 		test("handles complex keys", () => {
// 			expect(serializeKey(["a,b", EMPTY_KEY, "c,d"])).toBe(
// 				`a\\,b${KEY_SEPARATOR}\\${EMPTY_KEY}${KEY_SEPARATOR}c\\,d`,
// 			);
// 		});
// 	});
//
// 	// Test key deserialization
// 	describe("deserializeKey", () => {
// 		test("deserializes empty string", () => {
// 			expect(deserializeKey("")).toEqual([]);
// 		});
//
// 		test("deserializes undefined/null", () => {
// 			expect(deserializeKey(undefined as unknown as string)).toEqual([]);
// 			expect(deserializeKey(null as unknown as string)).toEqual([]);
// 		});
//
// 		test("deserializes empty key marker", () => {
// 			expect(deserializeKey(EMPTY_KEY)).toEqual([]);
// 		});
//
// 		test("deserializes single key", () => {
// 			expect(deserializeKey("test")).toEqual(["test"]);
// 		});
//
// 		test("deserializes multiple keys", () => {
// 			expect(
// 				deserializeKey(`a${KEY_SEPARATOR}b${KEY_SEPARATOR}c`),
// 			).toEqual(["a", "b", "c"]);
// 		});
//
// 		test("deserializes keys with escaped commas", () => {
// 			expect(deserializeKey("a\\,b")).toEqual(["a,b"]);
// 			expect(deserializeKey(`a\\,b${KEY_SEPARATOR}c`)).toEqual([
// 				"a,b",
// 				"c",
// 			]);
// 		});
//
// 		test("deserializes keys with escaped empty key marker", () => {
// 			expect(deserializeKey(`\\${EMPTY_KEY}`)).toEqual([EMPTY_KEY]);
// 		});
//
// 		test("deserializes complex keys", () => {
// 			expect(
// 				deserializeKey(
// 					`a\\,b${KEY_SEPARATOR}\\${EMPTY_KEY}${KEY_SEPARATOR}c\\,d`,
// 				),
// 			).toEqual(["a,b", EMPTY_KEY, "c,d"]);
// 		});
// 	});
//
// 	// Test name+key serialization
// 	describe("serializeNameAndKey", () => {
// 		test("serializes name with empty key array", () => {
// 			expect(serializeNameAndKey("test", [])).toBe(`test:${EMPTY_KEY}`);
// 		});
//
// 		test("serializes name with single key", () => {
// 			expect(serializeNameAndKey("test", ["key1"])).toBe("test:key1");
// 		});
//
// 		test("serializes name with multiple keys", () => {
// 			expect(serializeNameAndKey("test", ["a", "b", "c"])).toBe(
// 				`test:a${KEY_SEPARATOR}b${KEY_SEPARATOR}c`,
// 			);
// 		});
//
// 		test("escapes commas in keys", () => {
// 			expect(serializeNameAndKey("test", ["a,b"])).toBe("test:a\\,b");
// 		});
//
// 		test("handles complex keys with name", () => {
// 			expect(
// 				serializeNameAndKey("actor", ["a,b", EMPTY_KEY, "c,d"]),
// 			).toBe(
// 				`actor:a\\,b${KEY_SEPARATOR}\\${EMPTY_KEY}${KEY_SEPARATOR}c\\,d`,
// 			);
// 		});
// 	});
//
// 	// Removed createIndexKey tests as function was moved to KEYS.INDEX in manager_driver.ts
//
// 	// Test roundtrip
// 	describe("roundtrip", () => {
// 		const testKeys = [
// 			[],
// 			["test"],
// 			["a", "b", "c"],
// 			["a,b", "c"],
// 			[EMPTY_KEY],
// 			["a,b", EMPTY_KEY, "c,d"],
// 			["special\\chars", "more:complex,keys", "final key"],
// 		];
//
// 		testKeys.forEach((key) => {
// 			test(`roundtrip: ${JSON.stringify(key)}`, () => {
// 				const serialized = serializeKey(key);
// 				const deserialized = deserializeKey(serialized);
// 				expect(deserialized).toEqual(key);
// 			});
// 		});
//
// 		test("handles all test cases in a large batch", () => {
// 			for (const key of testKeys) {
// 				const serialized = serializeKey(key);
// 				const deserialized = deserializeKey(serialized);
// 				expect(deserialized).toEqual(key);
// 			}
// 		});
// 	});
//
// 	// Test edge cases
// 	describe("edge cases", () => {
// 		test("handles backslash at the end", () => {
// 			const key = ["abc\\"];
// 			const serialized = serializeKey(key);
// 			const deserialized = deserializeKey(serialized);
// 			expect(deserialized).toEqual(key);
// 		});
//
// 		test("handles backslashes in middle of string", () => {
// 			const keys = [["abc\\def"], ["abc\\\\def"], ["path\\to\\file"]];
//
// 			for (const key of keys) {
// 				const serialized = serializeKey(key);
// 				const deserialized = deserializeKey(serialized);
// 				expect(deserialized).toEqual(key);
// 			}
// 		});
//
// 		test("handles commas at the end of strings", () => {
// 			const serialized = serializeKey(["abc\\,"]);
// 			expect(deserializeKey(serialized)).toEqual(["abc\\,"]);
// 		});
//
// 		test("handles mixed backslashes and commas", () => {
// 			const keys = [
// 				["path\\to\\file,dir"],
// 				["file\\with,comma"],
// 				["path\\to\\file", "with,comma"],
// 			];
//
// 			for (const key of keys) {
// 				const serialized = serializeKey(key);
// 				const deserialized = deserializeKey(serialized);
// 				expect(deserialized).toEqual(key);
// 			}
// 		});
//
// 		test("handles multiple consecutive commas", () => {
// 			const key = ["a,,b"];
// 			const serialized = serializeKey(key);
// 			const deserialized = deserializeKey(serialized);
// 			expect(deserialized).toEqual(key);
// 		});
//
// 		test("handles special characters", () => {
// 			const key = ["a💻b", "c🔑d"];
// 			const serialized = serializeKey(key);
// 			const deserialized = deserializeKey(serialized);
// 			expect(deserialized).toEqual(key);
// 		});
// 	});
//
// 	// Test exact key matching
// 	describe("exact key matching", () => {
// 		test("differentiates [a,b] from [a,b,c]", () => {
// 			const key1 = ["a", "b"];
// 			const key2 = ["a", "b", "c"];
//
// 			const serialized1 = serializeKey(key1);
// 			const serialized2 = serializeKey(key2);
//
// 			expect(serialized1).not.toBe(serialized2);
// 		});
//
// 		test("differentiates [a,b] from [a]", () => {
// 			const key1 = ["a", "b"];
// 			const key2 = ["a"];
//
// 			const serialized1 = serializeKey(key1);
// 			const serialized2 = serializeKey(key2);
//
// 			expect(serialized1).not.toBe(serialized2);
// 		});
//
// 		test("differentiates [a,b] from [a:b]", () => {
// 			const key1 = ["a,b"];
// 			const key2 = ["a", "b"];
//
// 			const serialized1 = serializeKey(key1);
// 			const serialized2 = serializeKey(key2);
//
// 			expect(serialized1).not.toBe(serialized2);
// 			expect(deserializeKey(serialized1)).toEqual(key1);
// 			expect(deserializeKey(serialized2)).toEqual(key2);
// 		});
// 	});
// });
