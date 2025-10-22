import { describe, expect, test } from "vitest";
import {
	deserializeActorKey,
	EMPTY_KEY,
	KEY_SEPARATOR,
	serializeActorKey,
} from "./keys";

describe("Key serialization and deserialization", () => {
	// Test serialization
	describe("serializeActorKey", () => {
		test("serializes empty key array", () => {
			expect(serializeActorKey([])).toBe(EMPTY_KEY);
		});

		test("serializes single key", () => {
			expect(serializeActorKey(["test"])).toBe("test");
		});

		test("serializes multiple keys", () => {
			expect(serializeActorKey(["a", "b", "c"])).toBe(
				`a${KEY_SEPARATOR}b${KEY_SEPARATOR}c`,
			);
		});

		test("escapes forward slashes in keys", () => {
			expect(serializeActorKey(["a/b"])).toBe("a\\/b");
			expect(serializeActorKey(["a/b", "c"])).toBe(
				`a\\/b${KEY_SEPARATOR}c`,
			);
		});

		test("escapes empty key marker in keys", () => {
			expect(serializeActorKey([EMPTY_KEY])).toBe(`\\${EMPTY_KEY}`);
		});

		test("handles complex keys", () => {
			expect(serializeActorKey(["a/b", EMPTY_KEY, "c/d"])).toBe(
				`a\\/b${KEY_SEPARATOR}\\${EMPTY_KEY}${KEY_SEPARATOR}c\\/d`,
			);
		});
	});

	// Test deserialization
	describe("deserializeActorKey", () => {
		test("deserializes empty string", () => {
			expect(deserializeActorKey("")).toEqual([]);
		});

		test("deserializes undefined/null", () => {
			expect(deserializeActorKey(undefined as unknown as string)).toEqual(
				[],
			);
			expect(deserializeActorKey(null as unknown as string)).toEqual([]);
		});

		test("deserializes empty key marker", () => {
			expect(deserializeActorKey(EMPTY_KEY)).toEqual([]);
		});

		test("deserializes single key", () => {
			expect(deserializeActorKey("test")).toEqual(["test"]);
		});

		test("deserializes multiple keys", () => {
			expect(
				deserializeActorKey(`a${KEY_SEPARATOR}b${KEY_SEPARATOR}c`),
			).toEqual(["a", "b", "c"]);
		});

		test("deserializes keys with escaped forward slashes", () => {
			expect(deserializeActorKey("a\\/b")).toEqual(["a/b"]);
			expect(deserializeActorKey(`a\\/b${KEY_SEPARATOR}c`)).toEqual([
				"a/b",
				"c",
			]);
		});

		test("deserializes keys with escaped empty key marker", () => {
			expect(deserializeActorKey(`\\${EMPTY_KEY}`)).toEqual([EMPTY_KEY]);
		});

		test("deserializes complex keys", () => {
			expect(
				deserializeActorKey(
					`a\\/b${KEY_SEPARATOR}\\${EMPTY_KEY}${KEY_SEPARATOR}c\\/d`,
				),
			).toEqual(["a/b", EMPTY_KEY, "c/d"]);
		});
	});

	// Test edge cases
	describe("edge cases", () => {
		test("handles empty string parts", () => {
			const testCases: Array<[string[], string]> = [
				[[""], "\\0"],
				[["a", "", "b"], "a/\\0/b"],
				[["", "a"], "\\0/a"],
				[["a", ""], "a/\\0"],
				[["", "", ""], "\\0/\\0/\\0"],
				[["", "a", "", "b", ""], "\\0/a/\\0/b/\\0"],
				[[], "/"],
				[["test"], "test"],
				[["a", "b", "c"], "a/b/c"],
				[["a/b", "c"], "a\\/b/c"],
				[[EMPTY_KEY], "\\/"],
				[["a/b", EMPTY_KEY, "c/d"], "a\\/b/\\//c\\/d"],
				[
					["special\\chars", "more:complex,keys", "final key"],
					"special\\\\chars/more:complex,keys/final key",
				],
			];

			for (const [key, expectedSerialized] of testCases) {
				const serialized = serializeActorKey(key);
				expect(serialized).toBe(expectedSerialized);
				const deserialized = deserializeActorKey(serialized);
				expect(deserialized).toEqual(key);
			}
		});

		test("differentiates empty array from array with empty string", () => {
			const emptyArray: string[] = [];
			const arrayWithEmptyString = [""];

			const serialized1 = serializeActorKey(emptyArray);
			const serialized2 = serializeActorKey(arrayWithEmptyString);

			expect(serialized1).toBe(EMPTY_KEY); // Should be "/"
			expect(serialized2).not.toBe(EMPTY_KEY); // Should NOT be "/"
			expect(serialized2).toBe("\\0"); // Empty string becomes \0 marker

			expect(deserializeActorKey(serialized1)).toEqual(emptyArray);
			expect(deserializeActorKey(serialized2)).toEqual(
				arrayWithEmptyString,
			);
		});

		test("handles mix of empty strings and forward slash (EMPTY_KEY)", () => {
			const testCases: Array<[string[], string]> = [
				[["", EMPTY_KEY, ""], "\\0/\\//\\0"],
				[[EMPTY_KEY, ""], "\\//\\0"],
				[["", EMPTY_KEY], "\\0/\\/"],
				[["a", "", EMPTY_KEY, "", "b"], "a/\\0/\\//\\0/b"],
			];

			for (const [key, expectedSerialized] of testCases) {
				const serialized = serializeActorKey(key);
				expect(serialized).toBe(expectedSerialized);
				const deserialized = deserializeActorKey(serialized);
				expect(deserialized).toEqual(key);
			}
		});

		test("handles literal backslash-zero string", () => {
			const testCases: Array<[string[], string]> = [
				[["\\0"], "\\\\0"], // Literal \0 string
				[["a\\0b"], "a\\\\0b"], // Literal \0 in middle
				[["\\0", ""], "\\\\0/\\0"], // Literal \0 with empty string
				[["", "\\0"], "\\0/\\\\0"], // Empty string with literal \0
			];

			for (const [key, expectedSerialized] of testCases) {
				const serialized = serializeActorKey(key);
				expect(serialized).toBe(expectedSerialized);
				const deserialized = deserializeActorKey(serialized);
				expect(deserialized).toEqual(key);
			}
		});

		test("handles backslash at the end", () => {
			const key = ["abc\\"];
			const serialized = serializeActorKey(key);
			expect(serialized).toBe("abc\\\\");
			const deserialized = deserializeActorKey(serialized);
			expect(deserialized).toEqual(key);
		});

		test("handles backslashes in middle of string", () => {
			const testCases: Array<[string[], string]> = [
				[["abc\\def"], "abc\\\\def"],
				[["abc\\\\def"], "abc\\\\\\\\def"],
				[["path\\to\\file"], "path\\\\to\\\\file"],
			];

			for (const [key, expectedSerialized] of testCases) {
				const serialized = serializeActorKey(key);
				expect(serialized).toBe(expectedSerialized);
				const deserialized = deserializeActorKey(serialized);
				expect(deserialized).toEqual(key);
			}
		});

		test("handles forward slashes at the end of strings", () => {
			const serialized = serializeActorKey(["abc\\/"]);
			expect(deserializeActorKey(serialized)).toEqual(["abc\\/"]);
		});

		test("handles mixed backslashes and forward slashes", () => {
			const testCases: Array<[string[], string]> = [
				[["path\\to\\file/dir"], "path\\\\to\\\\file\\/dir"],
				[["file\\with/slash"], "file\\\\with\\/slash"],
				[
					["path\\to\\file", "with/slash"],
					"path\\\\to\\\\file/with\\/slash",
				],
			];

			for (const [key, expectedSerialized] of testCases) {
				const serialized = serializeActorKey(key);
				expect(serialized).toBe(expectedSerialized);
				const deserialized = deserializeActorKey(serialized);
				expect(deserialized).toEqual(key);
			}
		});

		test("handles multiple consecutive forward slashes", () => {
			const key = ["a//b"];
			const serialized = serializeActorKey(key);
			expect(serialized).toBe("a\\/\\/b");
			const deserialized = deserializeActorKey(serialized);
			expect(deserialized).toEqual(key);
		});

		test("handles special characters", () => {
			const key = ["a💻b", "c🔑d"];
			const serialized = serializeActorKey(key);
			expect(serialized).toBe("a💻b/c🔑d");
			const deserialized = deserializeActorKey(serialized);
			expect(deserialized).toEqual(key);
		});

		test("handles escaped forward slashes immediately after separator", () => {
			const key = ["abc", "/def"];
			const serialized = serializeActorKey(key);
			expect(serialized).toBe(`abc${KEY_SEPARATOR}\\/def`);
			expect(deserializeActorKey(serialized)).toEqual(key);
		});
	});

	// Test exact key matching
	describe("exact key matching", () => {
		test("differentiates [a,b] from [a,b,c]", () => {
			const key1 = ["a", "b"];
			const key2 = ["a", "b", "c"];

			const serialized1 = serializeActorKey(key1);
			const serialized2 = serializeActorKey(key2);

			expect(serialized1).not.toBe(serialized2);
		});

		test("differentiates [a,b] from [a]", () => {
			const key1 = ["a", "b"];
			const key2 = ["a"];

			const serialized1 = serializeActorKey(key1);
			const serialized2 = serializeActorKey(key2);

			expect(serialized1).not.toBe(serialized2);
		});

		test("differentiates [a/b] from [a,b]", () => {
			const key1 = ["a/b"];
			const key2 = ["a", "b"];

			const serialized1 = serializeActorKey(key1);
			const serialized2 = serializeActorKey(key2);

			expect(serialized1).not.toBe(serialized2);
			expect(deserializeActorKey(serialized1)).toEqual(key1);
			expect(deserializeActorKey(serialized2)).toEqual(key2);
		});
	});
});
