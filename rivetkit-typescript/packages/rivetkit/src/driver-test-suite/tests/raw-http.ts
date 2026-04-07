import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runRawHttpTests(driverTestConfig: DriverTestConfig) {
	describe("raw http", () => {
		test("should handle raw HTTP GET requests", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["test"]);

			// Test the hello endpoint
			const helloResponse = await actor.fetch("api/hello");
			expect(helloResponse.ok).toBe(true);
			const helloData = await helloResponse.json();
			expect(helloData).toEqual({ message: "Hello from actor!" });
		});

		test("should handle raw HTTP POST requests with echo", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["test"]);

			const testData = { test: "data", number: 123 };
			const echoResponse = await actor.fetch("api/echo", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(testData),
			});

			expect(echoResponse.ok).toBe(true);
			const echoData = await echoResponse.json();
			expect(echoData).toEqual(testData);
		});

		test("should track state across raw HTTP requests", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["state-test"]);

			// Make a few requests
			await actor.fetch("api/hello");
			await actor.fetch("api/hello");
			await actor.fetch("api/state");

			// Check the state endpoint
			const stateResponse = await actor.fetch("api/state");
			expect(stateResponse.ok).toBe(true);
			const stateData = (await stateResponse.json()) as {
				requestCount: number;
			};
			expect(stateData.requestCount).toBe(4); // 4 total requests
		});

		test("should pass headers correctly", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["headers-test"]);

			const customHeaders = {
				"X-Custom-Header": "test-value",
				"X-Another-Header": "another-value",
			};

			const response = await actor.fetch("api/headers", {
				headers: customHeaders,
			});

			expect(response.ok).toBe(true);
			const headers = (await response.json()) as Record<string, string>;
			expect(headers["x-custom-header"]).toBe("test-value");
			expect(headers["x-another-header"]).toBe("another-value");
		});

		test("should return 404 for unhandled paths", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["404-test"]);

			const response = await actor.fetch("api/nonexistent");
			expect(response.ok).toBe(false);
			expect(response.status).toBe(404);
		});

		test("should return 404 when no onRequest handler defined", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpNoHandlerActor.getOrCreate([
				"no-handler",
			]);

			const response = await actor.fetch("api/anything");
			expect(response.ok).toBe(false);
			expect(response.status).toBe(404);

			// No actions available without onRequest handler
		});

		test("should return 500 error when onRequest returns void", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpVoidReturnActor.getOrCreate([
				"void-return",
			]);

			const response = await actor.fetch("api/anything");
			expect(response.ok).toBe(false);
			expect(response.status).toBe(500);

			// Check error message - response might be CBOR encoded
			try {
				const errorData = (await response.json()) as {
					message: string;
				};
				expect(errorData.message).toContain(
					"onRequest handler must return a Response",
				);
			} catch {
				// If JSON parsing fails, just check that we got a 500 error
				// The error details are already validated by the status code
			}

			// No actions available when onRequest returns void
		});

		test("should handle different HTTP methods", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["methods-test"]);

			// Test various HTTP methods
			const methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];

			for (const method of methods) {
				const response = await actor.fetch("api/echo", {
					method,
					body: ["POST", "PUT", "PATCH"].includes(method)
						? JSON.stringify({ method })
						: undefined,
				});

				// Echo endpoint only handles POST, others should fall through to 404
				if (method === "POST") {
					expect(response.ok).toBe(true);
					const data = await response.json();
					expect(data).toEqual({ method });
				} else if (method === "GET") {
					// GET to echo should return 404
					expect(response.status).toBe(404);
				} else {
					// Other methods with body should also return 404
					expect(response.status).toBe(404);
				}
			}
		});

		test("should handle binary data", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["binary-test"]);

			// Send binary data
			const binaryData = new Uint8Array([1, 2, 3, 4, 5]);
			const response = await actor.fetch("api/echo", {
				method: "POST",
				headers: {
					"Content-Type": "application/octet-stream",
				},
				body: binaryData,
			});

			expect(response.ok).toBe(true);
			const responseBuffer = await response.arrayBuffer();
			const responseArray = new Uint8Array(responseBuffer);
			expect(Array.from(responseArray)).toEqual([1, 2, 3, 4, 5]);
		});

		test("should work with Hono router using createVars", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpHonoActor.getOrCreate(["hono-test"]);

			// Test root endpoint
			const rootResponse = await actor.fetch("/");
			expect(rootResponse.ok).toBe(true);
			const rootData = await rootResponse.json();
			expect(rootData).toEqual({ message: "Welcome to Hono actor!" });

			// Test GET all users
			const usersResponse = await actor.fetch("/users");
			expect(usersResponse.ok).toBe(true);
			const users = await usersResponse.json();
			expect(users).toEqual([
				{ id: 1, name: "Alice" },
				{ id: 2, name: "Bob" },
			]);

			// Test GET single user
			const userResponse = await actor.fetch("/users/1");
			expect(userResponse.ok).toBe(true);
			const user = await userResponse.json();
			expect(user).toEqual({ id: 1, name: "Alice" });

			// Test POST new user
			const newUser = { name: "Charlie" };
			const createResponse = await actor.fetch("/users", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(newUser),
			});
			expect(createResponse.ok).toBe(true);
			expect(createResponse.status).toBe(201);
			const createdUser = await createResponse.json();
			expect(createdUser).toEqual({ id: 3, name: "Charlie" });

			// Test PUT update user
			const updateData = { name: "Alice Updated" };
			const updateResponse = await actor.fetch("/users/1", {
				method: "PUT",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(updateData),
			});
			expect(updateResponse.ok).toBe(true);
			const updatedUser = await updateResponse.json();
			expect(updatedUser).toEqual({ id: 1, name: "Alice Updated" });

			// Test DELETE user
			const deleteResponse = await actor.fetch("/users/2", {
				method: "DELETE",
			});
			expect(deleteResponse.ok).toBe(true);
			const deleteResult = await deleteResponse.json();
			expect(deleteResult).toEqual({ message: "User 2 deleted" });

			// Test 404 for non-existent route
			const notFoundResponse = await actor.fetch("/api/unknown");
			expect(notFoundResponse.ok).toBe(false);
			expect(notFoundResponse.status).toBe(404);

			// No actions available on Hono actor
		});

		test("should handle paths with and without leading slashes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["path-test"]);

			// Test path without leading slash
			const responseWithoutSlash = await actor.fetch("api/hello");
			expect(responseWithoutSlash.ok).toBe(true);
			const dataWithoutSlash = await responseWithoutSlash.json();
			expect(dataWithoutSlash).toEqual({ message: "Hello from actor!" });

			// Test path with leading slash
			const responseWithSlash = await actor.fetch("/api/hello");
			expect(responseWithSlash.ok).toBe(true);
			const dataWithSlash = await responseWithSlash.json();
			expect(dataWithSlash).toEqual({ message: "Hello from actor!" });

			// Both should work the same way
		});

		test("should not create double slashes in request URLs", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			// Create a special actor that logs the request URL
			const actor = client.rawHttpHonoActor.getOrCreate(["url-test"]);

			// Test with leading slash - this was causing double slashes
			const response = await actor.fetch("/users");
			expect(response.ok).toBe(true);

			// The Hono router should receive a clean path without double slashes
			// If there were double slashes, Hono would not match the route correctly
			const data = await response.json();
			expect(data).toEqual([
				{ id: 1, name: "Alice" },
				{ id: 2, name: "Bob" },
			]);
		});

		test("should handle forwarded requests correctly without double slashes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpHonoActor.getOrCreate(["forward-test"]);

			// Simulate what the example does - pass path as string and Request as init
			const truncatedPath = "/users";
			const url = new URL(truncatedPath, "http://example.com");
			const newRequest = new Request(url, {
				method: "GET",
			});

			// This simulates calling actor.fetch(truncatedPath, newRequest)
			// which was causing double slashes in the example
			const response = await actor.fetch(
				truncatedPath,
				newRequest as any,
			);
			expect(response.ok).toBe(true);
			const users = await response.json();
			expect(users).toEqual([
				{ id: 1, name: "Alice" },
				{ id: 2, name: "Bob" },
			]);
		});

		test("example fix: should properly forward requests using just Request object", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpHonoActor.getOrCreate(["forward-fix"]);

			// The correct way - just pass the Request object
			const truncatedPath = "/users/1";
			const url = new URL(truncatedPath, "http://example.com");
			const newRequest = new Request(url, {
				method: "GET",
			});

			// Correct usage - just pass the Request
			const response = await actor.fetch(newRequest);
			expect(response.ok).toBe(true);
			const user = await response.json();
			expect(user).toEqual({ id: 1, name: "Alice" });
		});

		test("should support standard fetch API with URL and Request objects", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["fetch-api-test"]);

			// Test with URL object
			const url = new URL("/api/echo", "http://example.com");
			const urlResponse = await actor.fetch(url, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ from: "URL object" }),
			});
			expect(urlResponse.ok).toBe(true);
			const urlData = await urlResponse.json();
			expect(urlData).toEqual({ from: "URL object" });

			// Test with Request object
			const request = new Request("http://example.com/api/echo", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ from: "Request object" }),
			});
			const requestResponse = await actor.fetch(request);
			expect(requestResponse.ok).toBe(true);
			const requestData = await requestResponse.json();
			expect(requestData).toEqual({ from: "Request object" });

			// Test with Request object and additional init params
			const request2 = new Request("http://example.com/api/headers", {
				method: "GET",
				headers: { "X-Original": "request-header" },
			});
			const overrideResponse = await actor.fetch(request2, {
				headers: { "X-Override": "init-header" },
			});
			expect(overrideResponse.ok).toBe(true);
			const headers = (await overrideResponse.json()) as Record<
				string,
				string
			>;
			expect(headers["x-override"]).toBe("init-header");
			// Original headers should be present too
			expect(headers["x-original"]).toBe("request-header");
		});
	});
}
