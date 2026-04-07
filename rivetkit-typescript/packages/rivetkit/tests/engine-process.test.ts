import { getEnginePackageNameForPlatform } from "@rivetkit/engine";
import { describe, expect, it } from "vitest";

describe("getEnginePackageNameForPlatform", () => {
	it("returns the linux package name", () => {
		expect(getEnginePackageNameForPlatform("linux", "x64")).toBe(
			"@rivetkit/engine-linux-x64-musl",
		);
	});

	it("returns the darwin arm64 package name", () => {
		expect(getEnginePackageNameForPlatform("darwin", "arm64")).toBe(
			"@rivetkit/engine-darwin-arm64",
		);
	});

	it("throws for unsupported platforms", () => {
		expect(() =>
			getEnginePackageNameForPlatform(
				"linux",
				"arm64" as typeof process.arch,
			),
		).toThrow(
			"unsupported platform for Rivet Engine npm package: linux/arm64",
		);
	});
});
