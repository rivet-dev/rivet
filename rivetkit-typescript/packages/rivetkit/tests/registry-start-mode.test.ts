import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { actor, setup } from "@/mod";
import { ENGINE_ENDPOINT } from "../src/common/engine";

const ENV_KEYS = [
	"NODE_ENV",
	"RIVET_ENDPOINT",
	"RIVET_ENGINE",
	"RIVET_ENVOY_VERSION",
	"RIVET_RUN_ENGINE",
] as const;

type BunGlobal = typeof globalThis & {
	Bun?: { env?: Record<string, string | undefined> };
};

const originalEnv = new Map<string, string | undefined>();
let originalBun: BunGlobal["Bun"];

const testActor = actor({
	state: {},
	actions: {},
});

function parseRegistryConfig(input: Partial<Parameters<typeof setup>[0]> = {}) {
	return setup({
		use: {
			test: testActor,
		},
		noWelcome: true,
		...input,
	}).parseConfig();
}

describe("registry start mode config", () => {
	beforeEach(() => {
		const global = globalThis as BunGlobal;
		originalBun = global.Bun;
		delete global.Bun;
		for (const key of ENV_KEYS) {
			originalEnv.set(key, process.env[key]);
			delete process.env[key];
		}
	});

	afterEach(() => {
		for (const key of ENV_KEYS) {
			const value = originalEnv.get(key);
			if (value === undefined) {
				delete process.env[key];
			} else {
				process.env[key] = value;
			}
		}
		originalEnv.clear();
		const global = globalThis as BunGlobal;
		if (originalBun === undefined) {
			delete global.Bun;
		} else {
			global.Bun = originalBun;
		}
	});

	test("defaults local development to a local engine envoy runtime", () => {
		const config = parseRegistryConfig();

		expect(config.startEngine).toBe(true);
		expect(config.runtimeMode).toBe("envoy");
		expect(config.endpoint).toBe(ENGINE_ENDPOINT);
	});

	test("production requires an endpoint and defaults to serverless", () => {
		process.env.NODE_ENV = "production";
		process.env.RIVET_ENVOY_VERSION = "1";

		expect(() => parseRegistryConfig()).toThrow(
			/Rivet endpoint is required when startEngine is false/,
		);

		const config = parseRegistryConfig({
			endpoint: "https://api.rivet.dev",
		});

		expect(config.startEngine).toBe(false);
		expect(config.runtimeMode).toBe("serverless");
		expect(config.endpoint).toBe("https://api.rivet.dev/");
	});

	test("Bun production env requires an endpoint", () => {
		(globalThis as BunGlobal).Bun = {
			env: {
				NODE_ENV: "production",
				RIVET_ENVOY_VERSION: "1",
			},
		};

		expect(() => parseRegistryConfig()).toThrow(
			/Rivet endpoint is required when startEngine is false/,
		);
	});

	test("configured endpoints default to serverless", () => {
		const config = parseRegistryConfig({
			endpoint: "https://api.rivet.dev",
		});

		expect(config.startEngine).toBe(false);
		expect(config.runtimeMode).toBe("serverless");
		expect(config.endpoint).toBe("https://api.rivet.dev/");
	});

	test("RIVET_ENDPOINT defaults to serverless", () => {
		process.env.RIVET_ENDPOINT = "https://api.rivet.dev";

		const config = parseRegistryConfig();

		expect(config.startEngine).toBe(false);
		expect(config.runtimeMode).toBe("serverless");
		expect(config.endpoint).toBe("https://api.rivet.dev/");
	});

	test("RIVET_ENGINE defaults to serverless", () => {
		process.env.RIVET_ENGINE = "https://api.rivet.dev";

		const config = parseRegistryConfig();

		expect(config.startEngine).toBe(false);
		expect(config.runtimeMode).toBe("serverless");
		expect(config.endpoint).toBe("https://api.rivet.dev/");
	});

	test("explicit envoy mode disables local engine startup", () => {
		const config = parseRegistryConfig({
			endpoint: "https://api.rivet.dev",
			mode: "envoy",
		});

		expect(config.startEngine).toBe(false);
		expect(config.runtimeMode).toBe("envoy");
		expect(config.endpoint).toBe("https://api.rivet.dev/");
	});

	test("explicit envoy mode requires an endpoint", () => {
		expect(() =>
			parseRegistryConfig({
				mode: "envoy",
			}),
		).toThrow(/Rivet endpoint is required when startEngine is false/);
	});

	test("explicit startEngine true rejects configured endpoints", () => {
		expect(() =>
			parseRegistryConfig({
				endpoint: "https://api.rivet.dev",
				startEngine: true,
			}),
		).toThrow(/cannot specify startEngine: true with a Rivet endpoint/);
	});

	test("explicit startEngine true overrides production defaults", () => {
		process.env.NODE_ENV = "production";
		process.env.RIVET_ENVOY_VERSION = "1";

		const config = parseRegistryConfig({
			startEngine: true,
		});

		expect(config.startEngine).toBe(true);
		expect(config.runtimeMode).toBe("envoy");
		expect(config.endpoint).toBe(ENGINE_ENDPOINT);
	});
});
