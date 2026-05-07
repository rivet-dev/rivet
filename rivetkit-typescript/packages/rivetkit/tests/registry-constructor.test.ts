import { actor, setup } from "@/mod";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { buildNativeRegistry } from "../src/registry/native";

const testActor = actor({
	state: {},
	actions: {},
});

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
		const initialTimeoutCalls = setTimeoutSpy.mock.calls.length;

		setup({
			use: {
				test: testActor,
			},
		});

		expect(setTimeoutSpy.mock.calls).toHaveLength(initialTimeoutCalls);

		await vi.runAllTimersAsync();
		expect(setTimeoutSpy.mock.calls).toHaveLength(initialTimeoutCalls);
	});

	test("reads config mutations made before the native registry is built", async () => {
		const setTimeoutSpy = vi.spyOn(globalThis, "setTimeout");
		const initialTimeoutCalls = setTimeoutSpy.mock.calls.length;

		const registry = setup({
			use: {
				test: testActor,
			},
			endpoint: "http://127.0.0.1:6642",
			token: "dev",
			namespace: "before-build",
			pool: "before-build-pool",
		});

		registry.config.namespace = "after-build";
		registry.config.endpoint = "http://127.0.0.1:7755";
		registry.config.pool = "after-build-pool";

		const { serveConfig } = await buildNativeRegistry(registry.parseConfig());

		expect(setTimeoutSpy.mock.calls).toHaveLength(initialTimeoutCalls);
		expect(new URL(serveConfig.endpoint).origin).toBe("http://127.0.0.1:7755");
		expect(serveConfig.namespace).toBe("after-build");
		expect(serveConfig.poolName).toBe("after-build-pool");
	});

	test("rejects multiple runtime entrypoints on one registry", () => {
		const registry = setup({
			use: {
				test: testActor,
			},
			endpoint: "http://127.0.0.1:6642",
			token: "dev",
			namespace: "entrypoint-guard",
			pool: "entrypoint-guard",
			version: 1,
			noWelcome: true,
		});

		registry.fetchHandler({ path: "/api/rivet" });

		expect(() => registry.start()).toThrow(
			/registry\.start\(\) cannot be used after registry\.fetchHandler\(\)/,
		);
		expect(() =>
			registry.listen({ port: 3000, path: "/api/rivet" }),
		).toThrow(
			/registry\.listen\(\) cannot be used after registry\.fetchHandler\(\)/,
		);
	});

	test.each(["noSleep", "onDestroyTimeout", "waitUntilTimeout"])(
		"rejects removed actor option %s",
		(option) => {
			expect(() =>
				actor({
					state: {},
					actions: {},
					options: {
						[option]: option === "noSleep" ? true : 1_000,
					},
				} as never),
			).toThrow(/unrecognized key/i);
		},
	);
});
