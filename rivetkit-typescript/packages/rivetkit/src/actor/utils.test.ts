import { describe, expect, test } from "vitest";
import { isConnStatePath, isStatePath } from "./utils";

describe("isStatePath", () => {
	test("matches exact state", () => {
		expect(isStatePath("state")).toBe(true);
	});

	test("matches nested state paths", () => {
		expect(isStatePath("state.foo")).toBe(true);
		expect(isStatePath("state.foo.bar")).toBe(true);
	});

	test("does not match other paths", () => {
		expect(isStatePath("connections")).toBe(false);
		expect(isStatePath("stateX")).toBe(false);
		expect(isStatePath("mystate")).toBe(false);
	});
});

describe("isConnStatePath", () => {
	test("matches connection state paths", () => {
		expect(isConnStatePath("connections.0.state")).toBe(true);
		expect(isConnStatePath("connections.123.state")).toBe(true);
	});

	test("matches nested connection state paths", () => {
		expect(isConnStatePath("connections.0.state.foo")).toBe(true);
		expect(isConnStatePath("connections.5.state.bar.baz")).toBe(true);
	});

	test("does not match non-state connection paths", () => {
		expect(isConnStatePath("connections.0.params")).toBe(false);
		expect(isConnStatePath("connections.0.token")).toBe(false);
		expect(isConnStatePath("connections.0")).toBe(false);
	});

	test("does not match other paths", () => {
		expect(isConnStatePath("state")).toBe(false);
		expect(isConnStatePath("connections")).toBe(false);
		expect(isConnStatePath("other.0.state")).toBe(false);
	});

	test("does not match malformed paths", () => {
		expect(isConnStatePath("connections.state")).toBe(false);
		expect(isConnStatePath("connections.0.stateX")).toBe(false);
	});
});
