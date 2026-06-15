import getPort from "get-port";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { actor, setup } from "@/mod";
import type { Registry } from "@/registry";
import { getRivetkitRuntimeMode, parsePortEnv } from "@/utils/env-vars";

describe("getRivetkitRuntimeMode", () => {
	let snapshot: string | undefined;

	beforeEach(() => {
		snapshot = process.env.RIVETKIT_RUNTIME_MODE;
		delete process.env.RIVETKIT_RUNTIME_MODE;
	});
	afterEach(() => {
		if (snapshot === undefined) delete process.env.RIVETKIT_RUNTIME_MODE;
		else process.env.RIVETKIT_RUNTIME_MODE = snapshot;
	});

	test("default (unset) is envoy", () => {
		expect(getRivetkitRuntimeMode()).toBe("envoy");
	});

	test("explicit serverless", () => {
		process.env.RIVETKIT_RUNTIME_MODE = "serverless";
		expect(getRivetkitRuntimeMode()).toBe("serverless");
	});

	test("explicit envoy", () => {
		process.env.RIVETKIT_RUNTIME_MODE = "envoy";
		expect(getRivetkitRuntimeMode()).toBe("envoy");
	});

	test("empty string is envoy", () => {
		process.env.RIVETKIT_RUNTIME_MODE = "";
		expect(getRivetkitRuntimeMode()).toBe("envoy");
	});

	test("unrecognized value is envoy", () => {
		process.env.RIVETKIT_RUNTIME_MODE = "potato";
		expect(getRivetkitRuntimeMode()).toBe("envoy");
	});
});

describe("parsePortEnv", () => {
	test("undefined input returns undefined", () => {
		expect(parsePortEnv(undefined)).toBeUndefined();
	});

	test("empty string returns undefined", () => {
		expect(parsePortEnv("")).toBeUndefined();
	});

	test("valid integer string parses", () => {
		expect(parsePortEnv("8080")).toBe(8080);
	});

	test("port 1 is accepted (lower bound)", () => {
		expect(parsePortEnv("1")).toBe(1);
	});

	test("port 65535 is accepted (upper bound)", () => {
		expect(parsePortEnv("65535")).toBe(65535);
	});

	test("port 0 is rejected", () => {
		expect(() => parsePortEnv("0")).toThrow(/RIVET_PORT env var must be/);
	});

	test("port 65536 is rejected", () => {
		expect(() => parsePortEnv("65536")).toThrow(
			/RIVET_PORT env var must be/,
		);
	});

	test("non-numeric input is rejected", () => {
		expect(() => parsePortEnv("notaport")).toThrow(
			/RIVET_PORT env var must be/,
		);
	});

	test("partial numeric input is rejected (parseInt would silently succeed)", () => {
		expect(() => parsePortEnv("8080abc")).toThrow(
			/RIVET_PORT env var must be/,
		);
	});

	test("negative input is rejected", () => {
		expect(() => parsePortEnv("-1")).toThrow(/RIVET_PORT env var must be/);
	});
});

const testActor = actor({
	state: {},
	actions: {},
});

describe("registry.listen() end-to-end", () => {
	let registry: Registry<any> | undefined;
	let listenPromise: Promise<void> | undefined;

	afterEach(async () => {
		if (registry) {
			await registry.shutdown();
			registry = undefined;
		}
		if (listenPromise) {
			await listenPromise.catch(() => undefined);
			listenPromise = undefined;
		}
	}, 30_000);

	test("binds the requested port and serves /api/rivet/metadata", async () => {
		const port = await getPort({ host: "127.0.0.1" });
		registry = setup({
			use: { test: testActor },
			startEngine: false,
			endpoint: "http://127.0.0.1:65535",
			token: "dev",
			namespace: "default",
			noWelcome: true,
			shutdown: { disableSignalHandlers: true },
		}) as Registry<any>;

		listenPromise = registry.listen({ port, host: "127.0.0.1" });

		const baseUrl = `http://127.0.0.1:${port}`;
		const response = await waitForResponse(
			`${baseUrl}/api/rivet/metadata`,
			15_000,
		);
		expect(response.status).toBe(200);
		const body = await response.json();
		expect(body.runtime).toBe("rivetkit");
		expect(body.actorNames).toBeDefined();
		expect(body.actorNames).toHaveProperty("test");
	}, 30_000);

	test("/api/rivet/health returns ok", async () => {
		const port = await getPort({ host: "127.0.0.1" });
		registry = setup({
			use: { test: testActor },
			startEngine: false,
			endpoint: "http://127.0.0.1:65535",
			token: "dev",
			namespace: "default",
			noWelcome: true,
			shutdown: { disableSignalHandlers: true },
		}) as Registry<any>;

		listenPromise = registry.listen({ port, host: "127.0.0.1" });

		const response = await waitForResponse(
			`http://127.0.0.1:${port}/api/rivet/health`,
			15_000,
		);
		expect(response.status).toBe(200);
		const body = await response.json();
		expect(body.runtime).toBe("rivetkit");
		expect(body.status).toBeDefined();
	}, 30_000);
});

/**
 * Poll the URL until it responds (the listener takes a moment to bind and
 * build the serverless runtime on first request).
 */
async function waitForResponse(
	url: string,
	timeoutMs: number,
): Promise<Response> {
	const deadline = Date.now() + timeoutMs;
	let lastError: unknown;
	while (Date.now() < deadline) {
		try {
			const response = await fetch(url);
			return response;
		} catch (error) {
			lastError = error;
			await new Promise((resolve) => setTimeout(resolve, 100));
		}
	}
	throw new Error(`timed out waiting for ${url}: ${String(lastError)}`);
}
