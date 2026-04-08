import { describe, expect, it } from "vitest";
import {
	wrappingGteU16,
	wrappingGtU16,
	wrappingLteU16,
	wrappingLtU16,
} from "../src/utils";

describe("wrappingGtU16", () => {
	it("should return true when a > b in normal case", () => {
		expect(wrappingGtU16(100, 50)).toBe(true);
		expect(wrappingGtU16(1000, 999)).toBe(true);
	});

	it("should return false when a < b in normal case", () => {
		expect(wrappingGtU16(50, 100)).toBe(false);
		expect(wrappingGtU16(999, 1000)).toBe(false);
	});

	it("should return false when a == b", () => {
		expect(wrappingGtU16(100, 100)).toBe(false);
		expect(wrappingGtU16(0, 0)).toBe(false);
		expect(wrappingGtU16(65535, 65535)).toBe(false);
	});

	it("should handle wrapping around u16 max", () => {
		// When values wrap around, 1 is "greater than" 65535
		expect(wrappingGtU16(1, 65535)).toBe(true);
		expect(wrappingGtU16(100, 65500)).toBe(true);
	});

	it("should handle edge cases near u16 boundaries", () => {
		// 65535 is not greater than 0 (wrapped)
		expect(wrappingGtU16(65535, 0)).toBe(false);
		// But 0 is greater than 65535 if we consider wrapping
		expect(wrappingGtU16(0, 65535)).toBe(true);
	});

	it("should handle values at exactly half the range", () => {
		// U16_MAX / 2 = 32767.5, so values with distance <= 32767 return true
		const lessThanHalf = 32766;
		expect(wrappingGtU16(lessThanHalf, 0)).toBe(true);
		expect(wrappingGtU16(0, lessThanHalf)).toBe(false);

		// At distance 32767, still less than 32767.5, so comparison returns true
		const atHalfDistance = 32767;
		expect(wrappingGtU16(atHalfDistance, 0)).toBe(true);
		expect(wrappingGtU16(0, atHalfDistance)).toBe(false);

		// At distance 32768, greater than 32767.5, so comparison returns false
		const overHalfDistance = 32768;
		expect(wrappingGtU16(overHalfDistance, 0)).toBe(false);
		expect(wrappingGtU16(0, overHalfDistance)).toBe(false);
	});
});

describe("wrappingLtU16", () => {
	it("should return true when a < b in normal case", () => {
		expect(wrappingLtU16(50, 100)).toBe(true);
		expect(wrappingLtU16(999, 1000)).toBe(true);
	});

	it("should return false when a > b in normal case", () => {
		expect(wrappingLtU16(100, 50)).toBe(false);
		expect(wrappingLtU16(1000, 999)).toBe(false);
	});

	it("should return false when a == b", () => {
		expect(wrappingLtU16(100, 100)).toBe(false);
		expect(wrappingLtU16(0, 0)).toBe(false);
		expect(wrappingLtU16(65535, 65535)).toBe(false);
	});

	it("should handle wrapping around u16 max", () => {
		// When values wrap around, 65535 is "less than" 1
		expect(wrappingLtU16(65535, 1)).toBe(true);
		expect(wrappingLtU16(65500, 100)).toBe(true);
	});

	it("should handle edge cases near u16 boundaries", () => {
		// 0 is not less than 65535 (wrapped)
		expect(wrappingLtU16(0, 65535)).toBe(false);
		// But 65535 is less than 0 if we consider wrapping
		expect(wrappingLtU16(65535, 0)).toBe(true);
	});

	it("should handle values at exactly half the range", () => {
		// U16_MAX / 2 = 32767.5, so values with distance <= 32767 return true
		const lessThanHalf = 32766;
		expect(wrappingLtU16(0, lessThanHalf)).toBe(true);
		expect(wrappingLtU16(lessThanHalf, 0)).toBe(false);

		// At distance 32767, still less than 32767.5, so comparison returns true
		const atHalfDistance = 32767;
		expect(wrappingLtU16(0, atHalfDistance)).toBe(true);
		expect(wrappingLtU16(atHalfDistance, 0)).toBe(false);

		// At distance 32768, greater than 32767.5, so comparison returns false
		const overHalfDistance = 32768;
		expect(wrappingLtU16(0, overHalfDistance)).toBe(false);
		expect(wrappingLtU16(overHalfDistance, 0)).toBe(false);
	});
});

describe("wrappingGtU16 and wrappingLtU16 consistency", () => {
	it("should be inverse of each other for different values", () => {
		const testCases: [number, number][] = [
			[100, 200],
			[200, 100],
			[0, 65535],
			[65535, 0],
			[1, 65534],
			[32767, 32768],
		];

		for (const [a, b] of testCases) {
			const gt = wrappingGtU16(a, b);
			const lt = wrappingLtU16(a, b);
			const eq = a === b;

			// For any pair, exactly one of gt, lt, or eq should be true
			expect(Number(gt) + Number(lt) + Number(eq)).toBe(1);
		}
	});

	it("should satisfy transitivity for sequential values", () => {
		// If we have sequential indices, a < b < c should hold
		const a = 100;
		const b = 101;
		const c = 102;

		expect(wrappingLtU16(a, b)).toBe(true);
		expect(wrappingLtU16(b, c)).toBe(true);
		expect(wrappingLtU16(a, c)).toBe(true);
	});

	it("should handle sequence across wrap boundary", () => {
		// Test a sequence that wraps: 65534, 65535, 0, 1
		const values = [65534, 65535, 0, 1];

		for (let i = 0; i < values.length - 1; i++) {
			expect(wrappingLtU16(values[i], values[i + 1])).toBe(true);
			expect(wrappingGtU16(values[i + 1], values[i])).toBe(true);
		}
	});
});

describe("wrappingGteU16", () => {
	it("should return true when a > b", () => {
		expect(wrappingGteU16(100, 50)).toBe(true);
		expect(wrappingGteU16(1000, 999)).toBe(true);
	});

	it("should return true when a == b", () => {
		expect(wrappingGteU16(100, 100)).toBe(true);
		expect(wrappingGteU16(0, 0)).toBe(true);
		expect(wrappingGteU16(65535, 65535)).toBe(true);
	});

	it("should return false when a < b", () => {
		expect(wrappingGteU16(50, 100)).toBe(false);
		expect(wrappingGteU16(999, 1000)).toBe(false);
	});

	it("should handle wrapping around u16 max", () => {
		expect(wrappingGteU16(1, 65535)).toBe(true);
		expect(wrappingGteU16(100, 65500)).toBe(true);
		expect(wrappingGteU16(0, 65535)).toBe(true);
	});
});

describe("wrappingLteU16", () => {
	it("should return true when a < b", () => {
		expect(wrappingLteU16(50, 100)).toBe(true);
		expect(wrappingLteU16(999, 1000)).toBe(true);
	});

	it("should return true when a == b", () => {
		expect(wrappingLteU16(100, 100)).toBe(true);
		expect(wrappingLteU16(0, 0)).toBe(true);
		expect(wrappingLteU16(65535, 65535)).toBe(true);
	});

	it("should return false when a > b", () => {
		expect(wrappingLteU16(100, 50)).toBe(false);
		expect(wrappingLteU16(1000, 999)).toBe(false);
	});

	it("should handle wrapping around u16 max", () => {
		expect(wrappingLteU16(65535, 1)).toBe(true);
		expect(wrappingLteU16(65500, 100)).toBe(true);
		expect(wrappingLteU16(65535, 0)).toBe(true);
	});
});
