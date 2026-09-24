import { afterEach, describe, expect, test, vi } from "vitest";
import { actor } from "@/actor/mod";
import { Registry, type RegistryDeps } from "@/registry";
import { logger } from "@/registry/log";

const testActor = actor({ state: {}, actions: {} });

function createRegistry(version?: number) {
	return new Registry(
		{
			use: { test: testActor },
			startEngine: false,
			noWelcome: true,
			envoy: version === undefined ? undefined : { version },
			shutdown: { disableSignalHandlers: true },
		},
		{
			buildConfiguredRegistry: (() =>
				new Promise(
					() => {},
				)) as RegistryDeps["buildConfiguredRegistry"],
		},
	);
}

describe("envoy version startup logging", () => {
	afterEach(() => {
		vi.restoreAllMocks();
		vi.unstubAllEnvs();
	});

	test("logs the effective explicitly configured version", () => {
		vi.stubEnv("NODE_ENV", "production");
		vi.stubEnv("RIVET_ENVOY_VERSION", undefined);
		const info = vi.spyOn(logger(), "info").mockImplementation(() => {});
		const error = vi.spyOn(logger(), "error").mockImplementation(() => {});

		createRegistry(1_790_235_315).startEnvoy();

		expect(info).toHaveBeenCalledWith({
			msg: "starting rivetkit envoy",
			rivetkitVersion: expect.any(String),
			envoyVersion: 1_790_235_315,
			envoyVersionSource: "config",
		});
		expect(error).not.toHaveBeenCalled();
	});

	test("logs the effective environment version", () => {
		vi.stubEnv("NODE_ENV", "production");
		vi.stubEnv("RIVET_ENVOY_VERSION", "1790237524");
		const info = vi.spyOn(logger(), "info").mockImplementation(() => {});
		const error = vi.spyOn(logger(), "error").mockImplementation(() => {});

		createRegistry().startEnvoy();

		expect(info).toHaveBeenCalledWith(
			expect.objectContaining({
				envoyVersion: 1_790_237_524,
				envoyVersionSource: "environment",
			}),
		);
		expect(error).not.toHaveBeenCalled();
	});

	test("reports the effective static default only for a starting production envoy", () => {
		vi.stubEnv("NODE_ENV", "production");
		vi.stubEnv("RIVET_ENVOY_VERSION", undefined);
		const info = vi.spyOn(logger(), "info").mockImplementation(() => {});
		const error = vi.spyOn(logger(), "error").mockImplementation(() => {});

		createRegistry().startEnvoy();

		expect(info).toHaveBeenCalledWith(
			expect.objectContaining({
				envoyVersion: 1,
				envoyVersionSource: "default",
			}),
		);
		expect(error).toHaveBeenCalledWith(
			expect.objectContaining({
				envoyVersion: 1,
				envoyVersionSource: "default",
			}),
		);
	});
});
