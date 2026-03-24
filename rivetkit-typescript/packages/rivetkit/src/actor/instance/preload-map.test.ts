import { describe, it, expect } from "vitest";
import {
	compareBytes,
	binarySearch,
	buildPreloadMap,
	type PreloadedEntries,
	type PreloadedKvInput,
} from "./preload-map";

// Helper to create Uint8Array from a list of byte values.
function bytes(...values: number[]): Uint8Array {
	return new Uint8Array(values);
}

describe("compareBytes", () => {
	it("returns 0 for equal arrays", () => {
		expect(compareBytes(bytes(1, 2, 3), bytes(1, 2, 3))).toBe(0);
	});

	it("returns 0 for two empty arrays", () => {
		expect(compareBytes(bytes(), bytes())).toBe(0);
	});

	it("returns negative when first array is shorter", () => {
		expect(compareBytes(bytes(1, 2), bytes(1, 2, 3))).toBeLessThan(0);
	});

	it("returns positive when first array is longer", () => {
		expect(compareBytes(bytes(1, 2, 3), bytes(1, 2))).toBeGreaterThan(0);
	});

	it("compares lexicographically when bytes differ", () => {
		expect(compareBytes(bytes(1, 2), bytes(1, 3))).toBeLessThan(0);
		expect(compareBytes(bytes(1, 3), bytes(1, 2))).toBeGreaterThan(0);
	});

	it("compares first differing byte", () => {
		expect(compareBytes(bytes(5, 0, 0), bytes(3, 9, 9))).toBeGreaterThan(0);
	});

	it("handles single-byte arrays", () => {
		expect(compareBytes(bytes(0), bytes(0))).toBe(0);
		expect(compareBytes(bytes(0), bytes(1))).toBeLessThan(0);
		expect(compareBytes(bytes(255), bytes(0))).toBeGreaterThan(0);
	});
});

describe("binarySearch", () => {
	it("finds a key in a sorted entries array", () => {
		const entries: PreloadedEntries = [
			[bytes(1), bytes(10)],
			[bytes(2), bytes(20)],
			[bytes(3), bytes(30)],
		];
		const result = binarySearch(entries, bytes(2));
		expect(result).toEqual(bytes(20));
	});

	it("returns undefined when key is not found", () => {
		const entries: PreloadedEntries = [
			[bytes(1), bytes(10)],
			[bytes(3), bytes(30)],
		];
		expect(binarySearch(entries, bytes(2))).toBeUndefined();
	});

	it("returns undefined for an empty array", () => {
		expect(binarySearch([], bytes(1))).toBeUndefined();
	});

	it("finds the only element in a single-element array", () => {
		const entries: PreloadedEntries = [[bytes(5), bytes(50)]];
		expect(binarySearch(entries, bytes(5))).toEqual(bytes(50));
	});

	it("returns undefined when key is not in single-element array", () => {
		const entries: PreloadedEntries = [[bytes(5), bytes(50)]];
		expect(binarySearch(entries, bytes(3))).toBeUndefined();
	});

	it("finds the first element", () => {
		const entries: PreloadedEntries = [
			[bytes(1), bytes(10)],
			[bytes(2), bytes(20)],
			[bytes(3), bytes(30)],
		];
		expect(binarySearch(entries, bytes(1))).toEqual(bytes(10));
	});

	it("finds the last element", () => {
		const entries: PreloadedEntries = [
			[bytes(1), bytes(10)],
			[bytes(2), bytes(20)],
			[bytes(3), bytes(30)],
		];
		expect(binarySearch(entries, bytes(3))).toEqual(bytes(30));
	});

	it("handles multi-byte keys correctly", () => {
		const entries: PreloadedEntries = [
			[bytes(1, 0), bytes(10)],
			[bytes(1, 1), bytes(11)],
			[bytes(2, 0), bytes(20)],
		];
		expect(binarySearch(entries, bytes(1, 1))).toEqual(bytes(11));
		expect(binarySearch(entries, bytes(1, 2))).toBeUndefined();
	});
});

describe("buildPreloadMap", () => {
	it("returns undefined for null input", () => {
		expect(buildPreloadMap(null)).toBeUndefined();
	});

	it("returns undefined for undefined input", () => {
		expect(buildPreloadMap(undefined)).toBeUndefined();
	});

	describe("get()", () => {
		it("returns Uint8Array when key exists in entries", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(1).buffer, value: bytes(10).buffer },
					{ key: bytes(2).buffer, value: bytes(20).buffer },
				],
				requestedGetKeys: [bytes(1).buffer, bytes(2).buffer],
				requestedPrefixes: [],
			};
			const map = buildPreloadMap(input)!;
			expect(map).toBeDefined();
			expect(map.get(bytes(1))).toEqual(bytes(10));
			expect(map.get(bytes(2))).toEqual(bytes(20));
		});

		it("returns null when key is in requestedGetKeys but not in entries", () => {
			const input: PreloadedKvInput = {
				entries: [],
				requestedGetKeys: [bytes(1).buffer, bytes(5).buffer],
				requestedPrefixes: [],
			};
			const map = buildPreloadMap(input)!;
			expect(map.get(bytes(1))).toBeNull();
			expect(map.get(bytes(5))).toBeNull();
		});

		it("returns undefined when key is not in requestedGetKeys", () => {
			const input: PreloadedKvInput = {
				entries: [{ key: bytes(1).buffer, value: bytes(10).buffer }],
				requestedGetKeys: [bytes(1).buffer],
				requestedPrefixes: [],
			};
			const map = buildPreloadMap(input)!;
			// Key 99 was never requested.
			expect(map.get(bytes(99))).toBeUndefined();
		});

		it("distinguishes all three return types", () => {
			const input: PreloadedKvInput = {
				entries: [{ key: bytes(1).buffer, value: bytes(10).buffer }],
				requestedGetKeys: [bytes(1).buffer, bytes(2).buffer],
				requestedPrefixes: [],
			};
			const map = buildPreloadMap(input)!;

			// Key exists in entries: returns value.
			const found = map.get(bytes(1));
			expect(found).toBeInstanceOf(Uint8Array);
			expect(found).toEqual(bytes(10));

			// Key requested but not found: returns null.
			expect(map.get(bytes(2))).toBeNull();

			// Key not requested at all: returns undefined.
			expect(map.get(bytes(3))).toBeUndefined();
		});

		it("handles entries provided in unsorted order", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(3).buffer, value: bytes(30).buffer },
					{ key: bytes(1).buffer, value: bytes(10).buffer },
					{ key: bytes(2).buffer, value: bytes(20).buffer },
				],
				requestedGetKeys: [
					bytes(3).buffer,
					bytes(1).buffer,
					bytes(2).buffer,
				],
				requestedPrefixes: [],
			};
			const map = buildPreloadMap(input)!;
			expect(map.get(bytes(1))).toEqual(bytes(10));
			expect(map.get(bytes(2))).toEqual(bytes(20));
			expect(map.get(bytes(3))).toEqual(bytes(30));
		});
	});

	describe("listPrefix()", () => {
		it("returns entries matching prefix", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(1, 0).buffer, value: bytes(10).buffer },
					{ key: bytes(1, 1).buffer, value: bytes(11).buffer },
					{ key: bytes(2, 0).buffer, value: bytes(20).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [bytes(1).buffer],
			};
			const map = buildPreloadMap(input)!;
			const result = map.listPrefix(bytes(1));
			expect(result).toBeDefined();
			expect(result).toHaveLength(2);
			expect(result![0][0]).toEqual(bytes(1, 0));
			expect(result![0][1]).toEqual(bytes(10));
			expect(result![1][0]).toEqual(bytes(1, 1));
			expect(result![1][1]).toEqual(bytes(11));
		});

		it("returns empty array when prefix was requested but has no entries", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(2, 0).buffer, value: bytes(20).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [bytes(1).buffer],
			};
			const map = buildPreloadMap(input)!;
			const result = map.listPrefix(bytes(1));
			expect(result).toBeDefined();
			expect(result).toEqual([]);
		});

		it("returns undefined when prefix was not requested", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(1, 0).buffer, value: bytes(10).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [],
			};
			const map = buildPreloadMap(input)!;
			expect(map.listPrefix(bytes(1))).toBeUndefined();
		});

		it("returns multiple entries with the same prefix", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(5, 0).buffer, value: bytes(50).buffer },
					{ key: bytes(5, 1).buffer, value: bytes(51).buffer },
					{ key: bytes(5, 2).buffer, value: bytes(52).buffer },
					{ key: bytes(5, 3).buffer, value: bytes(53).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [bytes(5).buffer],
			};
			const map = buildPreloadMap(input)!;
			const result = map.listPrefix(bytes(5));
			expect(result).toHaveLength(4);
			expect(result![0][1]).toEqual(bytes(50));
			expect(result![3][1]).toEqual(bytes(53));
		});

		it("does not match entries that share a byte prefix but belong to a different requested prefix", () => {
			// Prefix [1] should not match entries with key [1, 5, ...] if
			// we are listing prefix [1, 5]. And vice versa.
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(1, 0).buffer, value: bytes(10).buffer },
					{ key: bytes(1, 5, 0).buffer, value: bytes(150).buffer },
					{ key: bytes(1, 5, 1).buffer, value: bytes(151).buffer },
					{ key: bytes(2, 0).buffer, value: bytes(20).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [bytes(1, 5).buffer],
			};
			const map = buildPreloadMap(input)!;
			const result = map.listPrefix(bytes(1, 5));
			expect(result).toBeDefined();
			expect(result).toHaveLength(2);
			expect(result![0][0]).toEqual(bytes(1, 5, 0));
			expect(result![1][0]).toEqual(bytes(1, 5, 1));
		});

		it("an exact key match counts as having that prefix", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(3).buffer, value: bytes(30).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [bytes(3).buffer],
			};
			const map = buildPreloadMap(input)!;
			const result = map.listPrefix(bytes(3));
			expect(result).toBeDefined();
			expect(result).toHaveLength(1);
			expect(result![0][0]).toEqual(bytes(3));
		});

		it("empty prefix matches all entries", () => {
			const input: PreloadedKvInput = {
				entries: [
					{ key: bytes(1).buffer, value: bytes(10).buffer },
					{ key: bytes(2).buffer, value: bytes(20).buffer },
				],
				requestedGetKeys: [],
				requestedPrefixes: [bytes().buffer],
			};
			const map = buildPreloadMap(input)!;
			const result = map.listPrefix(bytes());
			expect(result).toBeDefined();
			expect(result).toHaveLength(2);
		});
	});
});
