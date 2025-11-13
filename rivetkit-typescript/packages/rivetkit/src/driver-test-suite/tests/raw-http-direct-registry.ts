// TODO: re-expose this once we can have actor queries on the gateway
// import { describe, expect, test } from "vitest";
// import {
// 	HEADER_ACTOR_QUERY,
// 	HEADER_CONN_PARAMS,
// } from "@/actor/router-endpoints";
// import type { ActorQuery } from "@/manager/protocol/query";
// import type { DriverTestConfig } from "../mod";
// import { setupDriverTest } from "../utils";
//
// export function runRawHttpDirectRegistryTests(
// 	driverTestConfig: DriverTestConfig,
// ) {
// 	describe("raw http - direct registry access", () => {
// 		test("should handle direct fetch requests to registry with proper headers", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			// Build the actor query
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpActor",
// 					key: ["direct-test"],
// 				},
// 			};
//
// 			// Make a direct fetch request to the registry
// 			const response = await fetch(
// 				`${endpoint}/registry/actors/request/api/hello`,
// 				{
// 					method: "GET",
// 					headers: {
// 						[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 					},
// 				},
// 			);
//
// 			expect(response.ok).toBe(true);
// 			expect(response.status).toBe(200);
// 			const data = await response.json();
// 			expect(data).toEqual({ message: "Hello from actor!" });
// 		});
//
// 		test("should handle POST requests with body to registry", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpActor",
// 					key: ["direct-post-test"],
// 				},
// 			};
//
// 			const testData = { test: "direct", number: 456 };
// 			const response = await fetch(
// 				`${endpoint}/registry/actors/request/api/echo`,
// 				{
// 					method: "POST",
// 					headers: {
// 						[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 						"Content-Type": "application/json",
// 					},
// 					body: JSON.stringify(testData),
// 				},
// 			);
//
// 			expect(response.ok).toBe(true);
// 			expect(response.status).toBe(200);
// 			const data = await response.json();
// 			expect(data).toEqual(testData);
// 		});
//
// 		test("should pass custom headers through to actor", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpActor",
// 					key: ["direct-headers-test"],
// 				},
// 			};
//
// 			const customHeaders = {
// 				"X-Custom-Header": "direct-test-value",
// 				"X-Another-Header": "another-direct-value",
// 			};
//
// 			const response = await fetch(
// 				`${endpoint}/registry/actors/request/api/headers`,
// 				{
// 					method: "GET",
// 					headers: {
// 						[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 						...customHeaders,
// 					},
// 				},
// 			);
//
// 			expect(response.ok).toBe(true);
// 			const headers = (await response.json()) as Record<string, string>;
// 			expect(headers["x-custom-header"]).toBe("direct-test-value");
// 			expect(headers["x-another-header"]).toBe("another-direct-value");
// 		});
//
// 		test("should handle connection parameters for authentication", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpActor",
// 					key: ["direct-auth-test"],
// 				},
// 			};
//
// 			const connParams = { token: "test-auth-token", userId: "user123" };
//
// 			const response = await fetch(
// 				`${endpoint}/registry/actors/request/api/hello`,
// 				{
// 					method: "GET",
// 					headers: {
// 						[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 						[HEADER_CONN_PARAMS]: JSON.stringify(connParams),
// 					},
// 				},
// 			);
//
// 			expect(response.ok).toBe(true);
// 			const data = await response.json();
// 			expect(data).toEqual({ message: "Hello from actor!" });
// 		});
//
// 		test("should return 404 for actors without onFetch handler", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpNoHandlerActor",
// 					key: ["direct-no-handler"],
// 				},
// 			};
//
// 			const response = await fetch(
// 				`${endpoint}/registry/actors/request/api/anything`,
// 				{
// 					method: "GET",
// 					headers: {
// 						[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 					},
// 				},
// 			);
//
// 			expect(response.ok).toBe(false);
// 			expect(response.status).toBe(404);
// 		});
//
// 		test("should handle different HTTP methods", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpActor",
// 					key: ["direct-methods-test"],
// 				},
// 			};
//
// 			// Test various HTTP methods
// 			const methods = ["GET", "POST", "PUT", "DELETE", "PATCH"] as const;
//
// 			for (const method of methods) {
// 				const response = await fetch(
// 					`${endpoint}/registry/actors/request/api/echo`,
// 					{
// 						method,
// 						headers: {
// 							[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 							...(method !== "GET"
// 								? { "Content-Type": "application/json" }
// 								: {}),
// 						},
// 						body: ["POST", "PUT", "PATCH"].includes(method)
// 							? JSON.stringify({ method })
// 							: undefined,
// 					},
// 				);
//
// 				// Echo endpoint only handles POST, others should fall through to 404
// 				if (method === "POST") {
// 					expect(response.ok).toBe(true);
// 					const data = await response.json();
// 					expect(data).toEqual({ method });
// 				} else {
// 					expect(response.status).toBe(404);
// 				}
// 			}
// 		});
//
// 		test("should handle binary data", async (c) => {
// 			const { endpoint } = await setupDriverTest(c, driverTestConfig);
//
// 			const actorQuery: ActorQuery = {
// 				getOrCreateForKey: {
// 					name: "rawHttpActor",
// 					key: ["direct-binary-test"],
// 				},
// 			};
//
// 			// Send binary data
// 			const binaryData = new Uint8Array([1, 2, 3, 4, 5]);
// 			const response = await fetch(
// 				`${endpoint}/registry/actors/request/api/echo`,
// 				{
// 					method: "POST",
// 					headers: {
// 						[HEADER_ACTOR_QUERY]: JSON.stringify(actorQuery),
// 						"Content-Type": "application/octet-stream",
// 					},
// 					body: binaryData,
// 				},
// 			);
//
// 			expect(response.ok).toBe(true);
// 			const responseBuffer = await response.arrayBuffer();
// 			const responseArray = new Uint8Array(responseBuffer);
// 			expect(Array.from(responseArray)).toEqual([1, 2, 3, 4, 5]);
// 		});
// 	});
// }
