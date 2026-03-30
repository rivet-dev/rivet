import { beforeEach, describe, expect, test, vi } from "vitest";

const svelteMock = vi.hoisted(() => {
	const markerContexts = new Map<symbol, unknown>();
	const valueContexts = new Map<symbol, unknown>();

	return {
		markerContexts,
		valueContexts,
		reset() {
			markerContexts.clear();
			valueContexts.clear();
		},
	};
});

vi.mock("svelte", () => ({
	createContext: () => {
		const key = Symbol("rivet-context");
		return [
			() => svelteMock.valueContexts.get(key),
			(value: unknown) => {
				svelteMock.valueContexts.set(key, value);
				return value;
			},
		] as const;
	},
	hasContext: (key: symbol) => svelteMock.markerContexts.has(key),
	setContext: (key: symbol, value: unknown) => {
		svelteMock.markerContexts.set(key, value);
		return value;
	},
}));

import { createRivetContext } from "../context.js";

describe("createRivetContext", () => {
	beforeEach(() => {
		svelteMock.reset();
	});

	test("supports set/get/has for typed contexts", () => {
		const context = createRivetContext("TestRivet");
		const rivet = {
			useActor: vi.fn(),
			createReactiveActor: vi.fn(),
		} as never;

		expect(context.has()).toBe(false);
		expect(context.set(rivet)).toBe(rivet);
		expect(context.has()).toBe(true);
		expect(context.get()).toBe(rivet);
	});

	test("reports missing context with a descriptive error", () => {
		const context = createRivetContext("TestRivet");

		expect(() => context.get()).toThrow(
			'Context "TestRivet" not found. Create an app-local Rivet context and call TestRivet.set(...) or TestRivet.setup(...) in a parent layout.',
		);
	});
});
