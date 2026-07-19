import { describe, expect, it } from "vitest";
import { formatValue } from "./format-value";

describe("formatValue", () => {
	it("pretty-prints bigint and byte buffers", () => {
		const formatted = formatValue(
			{
				count: 42n,
				bytes: new Uint8Array([1, 2, 3]),
				buffer: new Uint8Array([4, 5]).buffer,
			},
			true,
		);
		expect(formatted).toContain('"$BigInt"');
		expect(formatted).toContain('"$Uint8Array"');
		expect(formatted).toContain('"$ArrayBuffer"');
		expect(formatted).toContain("\n");
	});

	it("renders unsupported root values deterministically", () => {
		expect(formatValue(undefined)).toBe("undefined");
		expect(formatValue(Symbol("test"))).toBe("Symbol(test)");
		expect(formatValue(function example() {})).toBe("[Function example]");
	});

	it("does not throw for circular values", () => {
		const circular: Record<string, unknown> = {};
		circular.self = circular;
		expect(formatValue(circular)).toBe("[Unserializable value]");
	});
});
