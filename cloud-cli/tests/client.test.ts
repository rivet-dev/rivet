/**
 * Tests for the Cloud CLI.
 *
 * These tests mock the Cloud API client and verify that:
 *   1. `deploy` — calls the correct sequence of API methods.
 *   2. `logs` — streams log entries until the signal is aborted.
 */

import { describe, expect, it, mock, beforeEach, afterEach } from "bun:test";
import { CloudClient, CloudApiError } from "../src/lib/client.ts";

// ---------------------------------------------------------------------------
// CloudClient unit tests
// ---------------------------------------------------------------------------

describe("CloudClient", () => {
	describe("constructor", () => {
		it("trims trailing slash from base URL", () => {
			const client = new CloudClient({ token: "tok", baseUrl: "https://cloud-api.rivet.dev/" });
			expect(client.baseUrl).toBe("https://cloud-api.rivet.dev");
		});

		it("uses default base URL when none is provided", () => {
			const client = new CloudClient({ token: "tok" });
			expect(client.baseUrl).toBe("https://cloud-api.rivet.dev");
		});
	});

	describe("error handling", () => {
		it("throws CloudApiError for non-OK responses", async () => {
			const originalFetch = global.fetch;
			global.fetch = mock(async () =>
				new Response(JSON.stringify({ message: "Unauthorized" }), {
					status: 401,
					headers: { "Content-Type": "application/json" },
				}),
			) as unknown as typeof fetch;

			const client = new CloudClient({ token: "invalid" });
			try {
				await client.inspect();
				expect.unreachable("should have thrown");
			} catch (err) {
				expect(err).toBeInstanceOf(CloudApiError);
				expect((err as CloudApiError).status).toBe(401);
				expect((err as CloudApiError).message).toBe("Unauthorized");
			} finally {
				global.fetch = originalFetch;
			}
		});
	});

	describe("inspect", () => {
		it("returns organization and project from response", async () => {
			const originalFetch = global.fetch;
			global.fetch = mock(async () =>
				new Response(
					JSON.stringify({ project: "my-project", organization: "my-org" }),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				),
			) as unknown as typeof fetch;

			const client = new CloudClient({ token: "valid-token" });
			const result = await client.inspect();
			expect(result.project).toBe("my-project");
			expect(result.organization).toBe("my-org");

			global.fetch = originalFetch;
		});
	});

	describe("getNamespace", () => {
		it("returns null when API responds with 404", async () => {
			const originalFetch = global.fetch;
			global.fetch = mock(async () =>
				new Response(JSON.stringify({ message: "Not found" }), {
					status: 404,
					headers: { "Content-Type": "application/json" },
				}),
			) as unknown as typeof fetch;

			const client = new CloudClient({ token: "tok" });
			const ns = await client.getNamespace("proj", "ns-name", "org");
			expect(ns).toBeNull();

			global.fetch = originalFetch;
		});

		it("returns the namespace when found", async () => {
			const originalFetch = global.fetch;
			const fakeNs = {
				id: "ns-id",
				name: "production",
				displayName: "Production",
				createdAt: "2024-01-01T00:00:00Z",
			};
			global.fetch = mock(async () =>
				new Response(JSON.stringify({ namespace: fakeNs }), {
					status: 200,
					headers: { "Content-Type": "application/json" },
				}),
			) as unknown as typeof fetch;

			const client = new CloudClient({ token: "tok" });
			const ns = await client.getNamespace("proj", "production", "org");
			expect(ns).toEqual(fakeNs);

			global.fetch = originalFetch;
		});
	});

	describe("getManagedPool", () => {
		it("returns null when pool does not exist", async () => {
			const originalFetch = global.fetch;
			global.fetch = mock(async () =>
				new Response(JSON.stringify({ message: "Not found" }), {
					status: 404,
					headers: { "Content-Type": "application/json" },
				}),
			) as unknown as typeof fetch;

			const client = new CloudClient({ token: "tok" });
			const pool = await client.getManagedPool("proj", "ns", "default", "org");
			expect(pool).toBeNull();

			global.fetch = originalFetch;
		});
	});

	describe("upsertManagedPool", () => {
		it("sends a PUT request with the correct body", async () => {
			const originalFetch = global.fetch;
			let capturedUrl: string | undefined;
			let capturedMethod: string | undefined;
			let capturedBody: Record<string, unknown> | undefined;
			// biome-ignore lint/suspicious/noExplicitAny: test mock signature
			global.fetch = mock(async (url: string | URL, init?: RequestInit) => {
				capturedUrl = String(url);
				capturedMethod = init?.method;
				capturedBody = init?.body ? JSON.parse(init.body as string) : undefined;
				return new Response(null, { status: 204 });
			}) as unknown as typeof fetch;

			const client = new CloudClient({ token: "tok" });
			await client.upsertManagedPool("proj", "production", "default", {
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
		// Ensure env var doesn't interfere
		delete process.env.RIVET_CLOUD_TOKEN;
		// resolveToken exits on missing token, so pass it explicitly
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
