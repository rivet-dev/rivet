/**
 * Tests for the Cloud CLI.
 *
 * These tests verify that:
 *   1. `createCloudClient` — creates an @rivet-gg/cloud RivetClient correctly.
 *   2. `resolveToken` — picks up the token from the CLI flag or env var.
 */

import { describe, expect, it, mock, afterEach } from "bun:test";
import { RivetClient, RivetError } from "@rivet-gg/cloud";
import { createCloudClient } from "../src/lib/client.ts";

// ---------------------------------------------------------------------------
// createCloudClient factory
// ---------------------------------------------------------------------------

describe("createCloudClient", () => {
	it("returns a RivetClient instance", () => {
		const client = createCloudClient({ token: "tok" });
		expect(client).toBeInstanceOf(RivetClient);
	});

	it("uses the provided base URL", () => {
		const client = createCloudClient({
			token: "tok",
			baseUrl: "https://custom-api.rivet.dev",
		});
		expect(client).toBeInstanceOf(RivetClient);
	});

	it("falls back to the default Cloud API URL when none is provided", () => {
		const client = createCloudClient({ token: "tok" });
		expect(client).toBeInstanceOf(RivetClient);
	});
});

// ---------------------------------------------------------------------------
// SDK error handling (RivetError)
// ---------------------------------------------------------------------------

describe("RivetClient error handling", () => {
	it("throws RivetError for non-OK responses", async () => {
		const originalFetch = global.fetch;
		global.fetch = mock(async () =>
			new Response(JSON.stringify({ message: "Unauthorized", code: "UNAUTHORIZED" }), {
				status: 401,
				headers: { "Content-Type": "application/json" },
			}),
		) as unknown as typeof fetch;

		const client = createCloudClient({ token: "invalid" });
		try {
			await client.apiTokens.inspect();
			expect.unreachable("should have thrown");
		} catch (err) {
			expect(err).toBeInstanceOf(RivetError);
			expect((err as RivetError).statusCode).toBe(401);
		} finally {
			global.fetch = originalFetch;
		}
	});

	it("returns project and organization from inspect()", async () => {
		const originalFetch = global.fetch;
		global.fetch = mock(async () =>
			new Response(
				JSON.stringify({ project: "my-project", organization: "my-org" }),
				{ status: 200, headers: { "Content-Type": "application/json" } },
			),
		) as unknown as typeof fetch;

		const client = createCloudClient({ token: "valid-token" });
		const result = await client.apiTokens.inspect();
		expect(result.project).toBe("my-project");
		expect(result.organization).toBe("my-org");

		global.fetch = originalFetch;
	});

	it("throws RivetError with statusCode 404 when namespace not found", async () => {
		const originalFetch = global.fetch;
		global.fetch = mock(async () =>
			new Response(JSON.stringify({ message: "Not found", code: "NOT_FOUND" }), {
				status: 404,
				headers: { "Content-Type": "application/json" },
			}),
		) as unknown as typeof fetch;

		const client = createCloudClient({ token: "tok" });
		try {
			await client.namespaces.get("proj", "ns-name", { org: "org" });
			expect.unreachable("should have thrown");
		} catch (err) {
			expect(err).toBeInstanceOf(RivetError);
			expect((err as RivetError).statusCode).toBe(404);
		} finally {
			global.fetch = originalFetch;
		}
	});

	it("sends a PUT request when upserting a managed pool", async () => {
		const originalFetch = global.fetch;
		let capturedUrl: string | undefined;
		let capturedMethod: string | undefined;
		let capturedBody: Record<string, unknown> | undefined;
		// biome-ignore lint/suspicious/noExplicitAny: test mock signature
		global.fetch = mock(async (url: string | URL, init?: RequestInit) => {
			capturedUrl = String(url);
			capturedMethod = init?.method;
			capturedBody = init?.body ? JSON.parse(init.body as string) : undefined;
			return new Response(
				JSON.stringify({
					managedPool: {
						name: "default",
						status: "ready",
						config: { displayName: "default", minCount: 1, maxCount: 5 },
					},
				}),
				{ status: 200, headers: { "Content-Type": "application/json" } },
			);
		}) as unknown as typeof fetch;

		const client = createCloudClient({ token: "tok" });
		await client.managedPools.upsert("proj", "production", "default", {
			org: "my-org",
			image: { repository: "proj/default", tag: "abc123" },
			minCount: 1,
			maxCount: 5,
		});

		expect(capturedMethod).toBe("PUT");
		expect(capturedUrl).toContain("/projects/proj/namespaces/production/managed-pools/default");
		const image = capturedBody?.image as Record<string, unknown> | undefined;
		expect(image?.tag).toBe("abc123");
		expect(capturedBody?.minCount).toBe(1);

		global.fetch = originalFetch;
	});
});

// ---------------------------------------------------------------------------
// auth helpers
// ---------------------------------------------------------------------------

describe("resolveToken", () => {
	const originalEnv = process.env.RIVET_CLOUD_TOKEN;

	afterEach(() => {
		if (originalEnv === undefined) {
			delete process.env.RIVET_CLOUD_TOKEN;
		} else {
			process.env.RIVET_CLOUD_TOKEN = originalEnv;
		}
	});

	it("returns the CLI-supplied token", async () => {
		const { resolveToken } = await import("../src/lib/auth.ts");
		delete process.env.RIVET_CLOUD_TOKEN;
		const tok = resolveToken("my-explicit-token");
		expect(tok).toBe("my-explicit-token");
	});

	it("falls back to RIVET_CLOUD_TOKEN env var", async () => {
		process.env.RIVET_CLOUD_TOKEN = "env-token";
		const { resolveToken } = await import("../src/lib/auth.ts");
		const tok = resolveToken(undefined);
		expect(tok).toBe("env-token");
	});
});

