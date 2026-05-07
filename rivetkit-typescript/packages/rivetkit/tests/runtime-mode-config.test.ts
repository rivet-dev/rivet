import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { actor } from "@/actor/mod";
import { RegistryConfigSchema } from "@/registry/config";

const ENV_KEYS = [
	"NODE_ENV",
	"RIVET_MODE",
	"RIVET_ENDPOINT",
	"RIVET_TOKEN",
	"RIVET_NAMESPACE",
	"RIVET_POOL",
	"RIVET_VERSION",
	"RIVET_ENVOY_KEY",
	"RIVET_RUNNER",
	"RIVET_RUNNER_FOO",
	"RIVET_ENVOY_VERSION",
	"RIVET_ENVOY_KIND",
	"RIVET_POOL_NAME",
	"RIVET_RUN_ENGINE",
	"RIVET_RUN_ENGINE_VERSION",
	"RIVET_TOTAL_SLOTS",
] as const;

const savedEnv = new Map<string, string | undefined>();

const testActor = actor({
	state: {},
	actions: {},
});

function parseConfig(input: Record<string, unknown> = {}) {
	return RegistryConfigSchema.parse({
		use: { test: testActor },
		...input,
	});
}

describe("runtime entrypoint config", () => {
	beforeEach(() => {
		for (const key of ENV_KEYS) {
			savedEnv.set(key, process.env[key]);
			delete process.env[key];
		}
	});

	afterEach(() => {
		for (const key of ENV_KEYS) {
			const value = savedEnv.get(key);
			if (value === undefined) {
				delete process.env[key];
			} else {
				process.env[key] = value;
			}
		}
		savedEnv.clear();
	});

	test("defaults to managed envoy outside production", () => {
		const config = parseConfig();

		expect(config.mode).toBe("envoy");
		expect(config.modeSource).toBe("default");
		expect(config.startEngine).toBe(true);
		expect(config.endpoint).toBe("http://127.0.0.1:6420");
		expect(config.pool).toBe("default");
		expect(config.version).toBe(1);
	});

	test("defaults to external envoy in production", () => {
		process.env.NODE_ENV = "production";
		process.env.RIVET_VERSION = "7";

		const config = parseConfig({
			engine: { endpoint: "https://example.com" },
		});

		expect(config.mode).toBe("envoy");
		expect(config.modeSource).toBe("default");
		expect(config.startEngine).toBe(false);
		expect(config.endpoint).toBe("https://example.com/");
		expect(config.version).toBe(7);
	});

	test("requires production version", () => {
		process.env.NODE_ENV = "production";

		expect(() => parseConfig()).toThrow(/RIVET_VERSION is required/);
	});

	test("parses fetch handler entrypoint config", () => {
		process.env.NODE_ENV = "development";

		const config = parseConfig({
			entrypoint: {
				kind: "serverless",
				startEngine: true,
				devServerless: { url: "http://127.0.0.1:3000/api/rivet" },
				serverless: {
					basePath: "/api/rivet",
					maxStartPayloadBytes: 4096,
				},
			},
		});

		expect(config.mode).toBe("serverless");
		expect(config.modeSource).toBe("entrypoint");
		expect(config.startEngine).toBe(true);
		expect(config.endpoint).toBe("http://127.0.0.1:6420");
		expect(config.devServerless).toEqual({
			url: "http://127.0.0.1:3000/api/rivet",
		});
		expect(config.serverless.basePath).toBe("/api/rivet");
		expect(config.serverless.maxStartPayloadBytes).toBe(4096);
	});

	test("production ignores serverless dev config", () => {
		process.env.NODE_ENV = "production";
		process.env.RIVET_VERSION = "7";
		process.env.RIVET_ENDPOINT = "https://example.com";

		const config = parseConfig({
			entrypoint: {
				kind: "serverless",
				startEngine: true,
				devServerless: { url: "http://127.0.0.1:3000/api/rivet" },
			},
		});

		expect(config.mode).toBe("serverless");
		expect(config.startEngine).toBe(false);
		expect(config.devServerless).toBeUndefined();
	});

	test("parses listen entrypoint defaults", () => {
		process.env.NODE_ENV = "development";

		const config = parseConfig({
			entrypoint: {
				kind: "listen",
				startEngine: true,
				devServerless: { url: "http://localhost:3000/api/rivet" },
				httpBasePath: "/api/rivet",
				httpPort: 3000,
				staticDir: "public",
			},
		});

		expect(config.mode).toBe("serverless");
		expect(config.startEngine).toBe(true);
		expect(config.httpBasePath).toBe("/api/rivet");
		expect(config.httpPort).toBe(3000);
		expect(config.staticDir).toBe("public");
	});

	test("parses env-only shared runtime values", () => {
		process.env.RIVET_ENDPOINT = "https://ns:token@example.com";
		process.env.RIVET_POOL = "workers";
		process.env.RIVET_VERSION = "9";

		const config = parseConfig({
			entrypoint: {
				kind: "envoy",
			},
		});

		expect(config.mode).toBe("envoy");
		expect(config.modeSource).toBe("entrypoint");
		expect(config.endpoint).toBe("https://example.com/");
		expect(config.namespace).toBe("ns");
		expect(config.token).toBe("token");
		expect(config.pool).toBe("workers");
		expect(config.version).toBe(9);
	});

	test.each([
		["RIVET_ENDPOINT", "engine.endpoint", { engine: { endpoint: "https://example.com" } }],
		["RIVET_TOKEN", "token", { token: "manual" }],
		["RIVET_NAMESPACE", "namespace", { namespace: "manual" }],
		["RIVET_POOL", "pool", { pool: "manual" }],
		["RIVET_VERSION", "version", { version: 1 }],
	])("rejects %s with manual %s", (envName, _field, input) => {
		process.env[envName] = envName === "RIVET_VERSION" ? "2" : "env";

		expect(() => parseConfig(input)).toThrow(
			new RegExp(`${envName}.*cannot both be set`),
		);
	});

	test("rejects manual start envoy key", () => {
		expect(() =>
			parseConfig({
				entrypoint: {
					kind: "envoy",
					envoy: { key: "manual" },
				},
			}),
		).toThrow(/envoy\.key has been removed/);
	});

	test.each([
		["RIVET_RUNNER", "RIVET_POOL"],
		["RIVET_RUNNER_FOO", "RIVET_POOL"],
		["RIVET_ENVOY_VERSION", "RIVET_VERSION"],
		["RIVET_ENVOY_KEY", "managed by RivetKit"],
		["RIVET_ENVOY_KIND", "removed"],
		["RIVET_MODE", "removed"],
		["RIVET_POOL_NAME", "RIVET_POOL"],
		["RIVET_RUN_ENGINE", "removed"],
		["RIVET_RUN_ENGINE_VERSION", "removed"],
		["RIVET_TOTAL_SLOTS", "removed"],
	])("rejects legacy env %s", (envName, expected) => {
		process.env[envName] = "legacy";

		expect(() => parseConfig()).toThrow(new RegExp(expected));
	});

	test.each([
		[{ mode: "serverless" }, /mode has been removed/],
		[{ startEngine: false }, /startEngine has been removed/],
		[{ staticDir: "public" }, /staticDir has been removed/],
		[{ httpBasePath: "/api/rivet" }, /httpBasePath has been removed/],
		[{ httpPort: 3000 }, /httpPort has been removed/],
		[{ httpHost: "127.0.0.1" }, /httpHost has been removed/],
		[{ devServerless: { url: "http://127.0.0.1" } }, /devServerless has been removed/],
		[{ serverless: { basePath: "/api/rivet" } }, /serverless has been removed/],
		[{ envoy: { key: "old" } }, /envoy has been removed/],
		[{ configurePool: { url: "http://127.0.0.1" } }, /configurePool/],
	])("rejects removed setup config", (input, expected) => {
		expect(() => parseConfig(input)).toThrow(expected);
	});
});
