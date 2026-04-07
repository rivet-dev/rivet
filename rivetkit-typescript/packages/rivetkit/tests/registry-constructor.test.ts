import { actor, setup } from "@/mod";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { Runtime } from "../runtime";

const testActor = actor({
	state: {},
	actions: {},
});

function createMockRuntime() {
	return {
		startRunner: vi.fn(),
		startServerless: vi.fn(),
		handleServerlessRequest: vi.fn(),
	} as any;
}

describe("Registry constructor", () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
		vi.restoreAllMocks();
	});

	test("does not schedule prestart when it is not explicitly enabled", async () => {
		const setTimeoutSpy = vi.spyOn(globalThis, "setTimeout");
		const runtimeCreateSpy = vi
			.spyOn(Runtime, "create")
			.mockResolvedValue(createMockRuntime());
		const initialTimeoutCalls = setTimeoutSpy.mock.calls.length;

		setup({
			use: {
				test: testActor,
			},
		});

		expect(setTimeoutSpy.mock.calls).toHaveLength(initialTimeoutCalls);

		await vi.runAllTimersAsync();
		expect(runtimeCreateSpy).not.toHaveBeenCalled();
	});

	test("reads config mutations made before the prestart tick", async () => {
		const setTimeoutSpy = vi.spyOn(globalThis, "setTimeout");
		const initialTimeoutCalls = setTimeoutSpy.mock.calls.length;
		let engineVersion: string | undefined;

		vi.spyOn(Runtime, "create").mockImplementation(async (registry) => {
			engineVersion = registry.parseConfig().engineVersion;

			return createMockRuntime();
		});

		const registry = setup({
			use: {
				test: testActor,
			},
			startEngine: true,
		});

		registry.config.engineVersion = "9.9.9-test";

		expect(setTimeoutSpy.mock.calls).toHaveLength(initialTimeoutCalls + 1);

		await vi.runAllTimersAsync();
		expect(engineVersion).toBe("9.9.9-test");
	});
});
