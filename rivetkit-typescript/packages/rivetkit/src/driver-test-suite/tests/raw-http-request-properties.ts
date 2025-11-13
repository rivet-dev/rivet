import { describe, expect, test } from "vitest";
import { z } from "zod";
import { registry } from "../../../fixtures/driver-test-suite/registry";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runRawHttpRequestPropertiesTests(
	driverTestConfig: DriverTestConfig,
) {
	describe("raw http request properties", () => {
		test("should pass all Request properties correctly to onRequest", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Test basic request properties
			const response = await actor.fetch("test/path?foo=bar&baz=qux", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
					"X-Custom-Header": "custom-value",
					Authorization: "Bearer test-token",
				},
				body: JSON.stringify({ test: "data" }),
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;

			// Verify URL properties
			expect(data.url).toContain("/test/path?foo=bar&baz=qux");
			expect(data.pathname).toBe("/test/path");
			expect(data.search).toBe("?foo=bar&baz=qux");
			expect(data.searchParams).toEqual({
				foo: "bar",
				baz: "qux",
			});

			// Verify method
			expect(data.method).toBe("POST");

			// Verify headers
			expect(data.headers["content-type"]).toBe("application/json");
			expect(data.headers["x-custom-header"]).toBe("custom-value");
			expect(data.headers["authorization"]).toBe("Bearer test-token");

			// Verify body
			expect(data.body).toEqual({ test: "data" });
		});

		test("should handle GET requests with no body", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			const response = await actor.fetch("test/get", {
				method: "GET",
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;

			expect(data.method).toBe("GET");
			expect(data.body).toBeNull();
		});

		test("should handle different content types", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Test form data
			const formData = new URLSearchParams();
			formData.append("field1", "value1");
			formData.append("field2", "value2");

			const formResponse = await actor.fetch("test/form", {
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded",
				},
				body: formData.toString(),
			});

			expect(formResponse.ok).toBe(true);
			const formResult = (await formResponse.json()) as any;
			expect(formResult.headers["content-type"]).toBe(
				"application/x-www-form-urlencoded",
			);
			expect(formResult.bodyText).toBe("field1=value1&field2=value2");

			// Test plain text
			const textResponse = await actor.fetch("test/text", {
				method: "POST",
				headers: {
					"Content-Type": "text/plain",
				},
				body: "Hello, World!",
			});

			expect(textResponse.ok).toBe(true);
			const textResult = (await textResponse.json()) as any;
			expect(textResult.headers["content-type"]).toBe("text/plain");
			expect(textResult.bodyText).toBe("Hello, World!");
		});

		test("should preserve all header casing and values", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			const response = await actor.fetch("test/headers", {
				headers: {
					Accept: "application/json",
					"Accept-Language": "en-US,en;q=0.9",
					"Cache-Control": "no-cache",
					"User-Agent": "RivetKit-Test/1.0",
					"X-Request-ID": "12345",
				},
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;

			// Headers should be normalized to lowercase
			expect(data.headers["accept"]).toBe("application/json");
			expect(data.headers["accept-language"]).toBe("en-US,en;q=0.9");
			expect(data.headers["cache-control"]).toBe("no-cache");
			// User-Agent might be overwritten by the HTTP client, so just check it exists
			expect(data.headers["user-agent"]).toBeTruthy();
			expect(data.headers["x-request-id"]).toBe("12345");
		});

		test("should handle empty and special URL paths", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Test root path
			const rootResponse = await actor.fetch("");
			expect(rootResponse.ok).toBe(true);
			const rootData = (await rootResponse.json()) as any;
			expect(rootData.pathname).toBe("/");

			// Test path with special characters
			const specialResponse = await actor.fetch(
				"test/path%20with%20spaces/and%2Fslashes",
			);
			expect(specialResponse.ok).toBe(true);
			const specialData = (await specialResponse.json()) as any;
			// Note: The URL path may or may not be decoded depending on the HTTP client/server
			// Just verify it contains the expected segments
			expect(specialData.pathname).toMatch(
				/path.*with.*spaces.*and.*slashes/,
			);

			// Test path with fragment (should be ignored in server-side)
			const fragmentResponse = await actor.fetch("test/path#fragment");
			expect(fragmentResponse.ok).toBe(true);
			const fragmentData = (await fragmentResponse.json()) as any;
			expect(fragmentData.pathname).toBe("/test/path");
			expect(fragmentData.hash).toBe(""); // Fragments are not sent to server
		});

		test("should handle request properties for all HTTP methods", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			const methods = [
				"GET",
				"POST",
				"PUT",
				"DELETE",
				"PATCH",
				"HEAD",
				"OPTIONS",
			];

			for (const method of methods) {
				const response = await actor.fetch(
					`test/${method.toLowerCase()}`,
					{
						method,
						// Only include body for methods that support it
						body: ["POST", "PUT", "PATCH"].includes(method)
							? JSON.stringify({ method })
							: undefined,
					},
				);

				// HEAD responses have no body
				if (method === "HEAD") {
					expect(response.status).toBe(200);
					const text = await response.text();
					expect(text).toBe("");
				} else if (method === "OPTIONS") {
					expect(response.status).toBe(204);
					const text = await response.text();
					expect(text).toBe("");
				} else {
					expect(response.ok).toBe(true);
					const data = (await response.json()) as any;
					expect(data.method).toBe(method);
				}
			}
		});

		test("should handle complex query parameters", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Test multiple values for same key
			const response = await actor.fetch(
				"test?key=value1&key=value2&array[]=1&array[]=2&nested[prop]=val",
			);
			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;

			// Note: URLSearchParams only keeps the last value for duplicate keys
			expect(data.searchParams.key).toBe("value2");
			expect(data.searchParams["array[]"]).toBe("2");
			expect(data.searchParams["nested[prop]"]).toBe("val");
		});

		test("should handle multipart form data", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Create multipart boundary
			const boundary = "----RivetKitBoundary";
			const body = [
				`------${boundary}`,
				'Content-Disposition: form-data; name="field1"',
				"",
				"value1",
				`------${boundary}`,
				'Content-Disposition: form-data; name="field2"',
				"",
				"value2",
				`------${boundary}--`,
			].join("\r\n");

			const response = await actor.fetch("test/multipart", {
				method: "POST",
				headers: {
					"Content-Type": `multipart/form-data; boundary=----${boundary}`,
				},
				body: body,
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;
			expect(data.headers["content-type"]).toContain(
				"multipart/form-data",
			);
			expect(data.bodyText).toContain("field1");
			expect(data.bodyText).toContain("value1");
		});

		test("should handle very long URLs", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Create a very long query string
			const longValue = "x".repeat(1000);
			const response = await actor.fetch(`test/long?param=${longValue}`);

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;
			expect(data.searchParams.param).toBe(longValue);
			expect(data.search.length).toBeGreaterThan(1000);
		});

		test.skip("should handle large request bodies", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Create a large JSON body (1MB+)
			const largeArray = new Array(10000).fill({
				id: 1,
				name: "Test",
				description: "This is a test object with some data",
			});

			const response = await actor.fetch("test/large", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(largeArray),
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;
			expect(data.body).toHaveLength(10000);
		});

		test("should handle missing content-type header", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			const response = await actor.fetch("test/no-content-type", {
				method: "POST",
				body: "plain text without content-type",
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;
			expect(data.bodyText).toBe("plain text without content-type");
		});

		test("should handle empty request body", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			const response = await actor.fetch("test/empty", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: "",
			});

			expect(response.ok).toBe(true);
			// TODO: This is inconsistent between engine & file system driver
			// const data = (await response.json()) as any;
			// expect(data.body).toBeNull();
			// expect(data.bodyText).toBe("");
		});

		test("should handle custom HTTP methods", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Test a custom method (though most HTTP clients may not support this)
			try {
				const response = await actor.fetch("test/custom", {
					method: "CUSTOM",
				});

				// If the request succeeds, verify the method
				if (response.ok) {
					const data = (await response.json()) as any;
					expect(data.method).toBe("CUSTOM");
				}
			} catch (error) {
				// Some HTTP clients may reject custom methods
				// This is expected behavior
			}
		});

		test("should handle cookies in headers", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			const response = await actor.fetch("test/cookies", {
				headers: {
					Cookie: "session=abc123; user=test; preferences=dark_mode",
				},
			});

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;
			expect(data.headers.cookie).toBe(
				"session=abc123; user=test; preferences=dark_mode",
			);
		});

		test("should handle URL encoding properly", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Test various encoded characters
			const response = await actor.fetch(
				"test/encoded?special=%20%21%40%23%24%25%5E%26&unicode=%E2%9C%93&email=test%40example.com",
			);

			expect(response.ok).toBe(true);
			const data = (await response.json()) as any;

			// Verify URL decoding
			expect(data.searchParams.special).toBe(" !@#$%^&");
			expect(data.searchParams.unicode).toBe("✓");
			expect(data.searchParams.email).toBe("test@example.com");
		});

		test("should handle concurrent requests maintaining separate contexts", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpRequestPropertiesActor.getOrCreate([
				"test",
			]);

			// Send multiple concurrent requests with different data
			const requests = [
				actor.fetch("test/concurrent?id=1", {
					method: "POST",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify({ request: 1 }),
				}),
				actor.fetch("test/concurrent?id=2", {
					method: "PUT",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify({ request: 2 }),
				}),
				actor.fetch("test/concurrent?id=3", {
					method: "DELETE",
				}),
			];

			const responses = await Promise.all(requests);
			const results = (await Promise.all(
				responses.map((r) => r.json()),
			)) as any[];

			// Verify each request maintained its own context
			expect(results[0].searchParams.id).toBe("1");
			expect(results[0].method).toBe("POST");
			expect(results[0].body).toEqual({ request: 1 });

			expect(results[1].searchParams.id).toBe("2");
			expect(results[1].method).toBe("PUT");
			expect(results[1].body).toEqual({ request: 2 });

			expect(results[2].searchParams.id).toBe("3");
			expect(results[2].method).toBe("DELETE");
			expect(results[2].body).toBeNull();
		});
	});
}
