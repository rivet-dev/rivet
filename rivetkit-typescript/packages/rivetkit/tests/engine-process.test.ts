import { describe, expect, it } from "vitest";
import { resolveTargetTripletFor } from "@/engine-process/mod";

describe("resolveTargetTripletFor", () => {
	it("returns darwin arm64 target", () => {
		expect(resolveTargetTripletFor("darwin", "arm64")).toEqual({
			targetTriplet: "aarch64-apple-darwin",
			extension: "",
		});
	});

	it("returns darwin x64 target", () => {
		expect(resolveTargetTripletFor("darwin", "x64")).toEqual({
			targetTriplet: "x86_64-apple-darwin",
			extension: "",
		});
	});

	it("returns linux x64 target", () => {
		expect(resolveTargetTripletFor("linux", "x64")).toEqual({
			targetTriplet: "x86_64-unknown-linux-musl",
			extension: "",
		});
	});

	it("returns windows x64 target", () => {
		expect(resolveTargetTripletFor("win32", "x64")).toEqual({
			targetTriplet: "x86_64-pc-windows-gnu",
			extension: ".exe",
		});
	});

	it("throws for unsupported combinations", () => {
		expect(() =>
			resolveTargetTripletFor("linux", "arm64" as typeof process.arch),
		).toThrow("unsupported platform for rivet engine binary: linux/arm64");
	});
});
