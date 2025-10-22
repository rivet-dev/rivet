import { describe, expect, test } from "vitest";
import { HEADER_ACTOR_QUERY } from "@/driver-helpers/mod";
import {
	createActorInspectorClient,
	createManagerInspectorClient,
} from "@/inspector/mod";
import type { ActorQuery } from "@/mod";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorInspectorTests(driverTestConfig: DriverTestConfig) {
	// TODO: Add back
	describe.skip("Actor Inspector Tests", () => {
		describe("Manager Inspector", () => {
			test("should respond to ping", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.ping.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual({ message: "pong" });
			});

			test("should get actors with pagination", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Create some actors first
				await client.counter.create(["test-actor-1"]);
				await client.counter.create(["test-actor-2"]);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.actors.$get({
					query: { limit: "1" },
				});
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual(
					expect.arrayContaining([
						expect.objectContaining({ key: ["test-actor-1"] }),
					]),
				);
				expect(data.length).toBe(1);
			});

			test("should get all actors with pagination", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actorKey1 = ["test-cursor-1"];
				const actorKey2 = ["test-cursor-2"];

				// Create some actors first
				await client.counter.create(actorKey1);
				await client.counter.create(actorKey2);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.actors.$get({
					query: { limit: "5" },
				});
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual(
					expect.arrayContaining([
						expect.objectContaining({
							id: expect.any(String),
							key: actorKey1,
						}),
						expect.objectContaining({
							id: expect.any(String),
							key: actorKey2,
						}),
					]),
				);
			});

			test("should handle invalid limit parameter", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.actors.$get({
					query: { limit: "0" },
				});
				expect(response.status).toBe(400);
			});

			test("should create a new actor", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.actors.$post({
					json: {
						name: "default",
						key: ["test-create-actor"],
						input: {},
					},
				});

				expect(response.status).toBe(201);
				const data = await response.json();
				expect(data).toEqual(
					expect.objectContaining({
						id: expect.any(String),
						name: "default",
						key: ["test-create-actor"],
					}),
				);
			});

			test("should get builds", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.builds.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual(
					expect.arrayContaining([
						expect.objectContaining({ name: expect.any(String) }),
					]),
				);
			});

			test("should get actor by id", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Create an actor and get its ID
				const handle = await client.counter.create(["test-get-by-id"]);
				const actorId = await handle.resolve();

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.actor[":id"].$get({
					param: { id: actorId },
				});
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toHaveProperty("id", actorId);
			});

			test("should return 404 for non-existent actor", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.actor[":id"].$get({
					param: { id: "non-existent-id" },
				});
				expect(response.status).toBe(404);

				const data = await response.json();
				expect(data).toEqual({ error: "Actor not found" });
			});

			test("should get bootstrap data", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Create at least one actor to ensure bootstrap has data
				// Create an actor and get its ID
				const handle = await client.counter.create(["test-bootstrap"]);
				await handle.resolve();

				const http = createManagerInspectorClient(
					`${endpoint}/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
						},
					},
				);

				const response = await http.bootstrap.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data.actors).toEqual(
					expect.arrayContaining([
						expect.objectContaining({
							key: ["test-bootstrap"],
							name: "counter",
						}),
					]),
				);
			});
		});

		describe("Actor Inspector", () => {
			test("should handle actor not found", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const actorId = "non-existing";

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.ping.$get();
				expect(response.ok).toBe(false);
			});
			test("should respond to ping", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create(["test-ping"]);
				const actorId = await handle.resolve();

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.ping.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual({ message: "pong" });
			});

			test("should get actor state", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create(["test-state"]);
				const actorId = await handle.resolve();

				// Increment the counter to set some state
				await handle.increment(5);

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.state.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual({
					enabled: true,
					state: expect.objectContaining({
						count: 5,
					}),
				});
			});

			test("should update actor state with replace", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create([
					"test-state-replace",
				]);
				const actorId = await handle.resolve();

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				// Replace the entire state
				const response = await http.state.$patch({
					json: {
						replace: { count: 10 },
					},
				});
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual({
					enabled: true,
					state: { count: 10 },
				});
			});

			test("should update actor state with patch", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create([
					"test-state-patch",
				]);
				const actorId = await handle.resolve();

				// Set initial state
				await handle.increment(3);

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				// Patch the state
				const response = await http.state.$patch({
					json: {
						patch: [
							{
								op: "replace",
								path: "/count",
								value: 7,
							},
						],
					},
				});
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual({
					enabled: true,
					state: expect.objectContaining({
						count: 7,
					}),
				});
			});

			test("should get actor connections", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create([
					"test-connections",
				]);
				const actorId = await handle.resolve();
				handle.connect();
				await handle.increment(10);

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.connections.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data.connections).toEqual(
					expect.arrayContaining([
						expect.objectContaining({
							id: expect.any(String),
						}),
					]),
				);
			});

			test("should get actor events", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create(["test-events"]);
				const actorId = await handle.resolve();

				handle.connect();
				await handle.increment(10);

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.events.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data.events).toEqual(
					expect.arrayContaining([
						expect.objectContaining({
							type: "broadcast",
							id: expect.any(String),
						}),
					]),
				);
			});

			test("should clear actor events", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create([
					"test-events-clear",
				]);
				const actorId = await handle.resolve();

				handle.connect();
				await handle.increment(10);

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				{
					const response = await http.events.$get();
					expect(response.status).toBe(200);

					const data = await response.json();
					expect(data.events).toEqual(
						expect.arrayContaining([
							expect.objectContaining({
								type: "broadcast",
								id: expect.any(String),
							}),
						]),
					);
				}

				const response = await http.events.clear.$post();
				expect(response.status).toBe(200);
			});

			test("should get actor rpcs", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create(["test-rpcs"]);
				const actorId = await handle.resolve();

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.rpcs.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				expect(data).toEqual(
					expect.objectContaining({
						rpcs: expect.arrayContaining(["increment", "getCount"]),
					}),
				);
			});

			// database is not officially supported yet
			test.skip("should get actor database info", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create(["test-db"]);
				const actorId = await handle.resolve();

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				const response = await http.db.$get();
				expect(response.status).toBe(200);

				const data = await response.json();
				// Database might be enabled or disabled depending on actor configuration
				expect(data).toHaveProperty("enabled");
				expect(typeof data.enabled).toBe("boolean");

				if (data.enabled) {
					expect(data).toHaveProperty("db");
					expect(Array.isArray(data.db)).toBe(true);
				} else {
					expect(data.db).toBe(null);
				}
			});

			test.skip("should execute database query when database is enabled", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = await client.counter.create(["test-db-query"]);
				const actorId = await handle.resolve();

				const http = createActorInspectorClient(
					`${endpoint}/actors/inspect`,
					{
						headers: {
							Authorization: `Bearer token`,
							[HEADER_ACTOR_QUERY]: JSON.stringify({
								getForId: { name: "counter", actorId },
							} satisfies ActorQuery),
						},
					},
				);

				// First check if database is enabled
				const dbInfoResponse = await http.db.$get();
				const dbInfo = await dbInfoResponse.json();

				if (dbInfo.enabled) {
					// Execute a simple query
					const queryResponse = await http.db.$post({
						json: {
							query: "SELECT 1 as test",
							params: [],
						},
					});
					expect(queryResponse.status).toBe(200);

					const queryData = await queryResponse.json();
					expect(queryData).toHaveProperty("result");
				} else {
					// If database is not enabled, the POST should return enabled: false
					const queryResponse = await http.db.$post({
						json: {
							query: "SELECT 1 as test",
							params: [],
						},
					});
					expect(queryResponse.status).toBe(200);

					const queryData = await queryResponse.json();
					expect(queryData).toEqual({ enabled: false });
				}
			});
		});
	});
}
