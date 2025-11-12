import { describe } from "vitest";
import type { DriverTestConfig } from "../mod";

export function runActorInspectorTests(driverTestConfig: DriverTestConfig) {
	// TODO: Add back
	describe.skip("Actor Inspector Tests", () => {
		// describe("Actor Inspector", () => {
		// 	test("should handle actor not found", async (c) => {
		// 		const { endpoint } = await setupDriverTest(c, driverTestConfig);
		// 		const actorId = "non-existing";
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.ping.$get();
		// 		expect(response.ok).toBe(false);
		// 	});
		// 	test("should respond to ping", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create(["test-ping"]);
		// 		const actorId = await handle.resolve();
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.ping.$get();
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data).toEqual({ message: "pong" });
		// 	});
		// 	test("should get actor state", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create(["test-state"]);
		// 		const actorId = await handle.resolve();
		// 		// Increment the counter to set some state
		// 		await handle.increment(5);
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.state.$get();
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data).toEqual({
		// 			enabled: true,
		// 			state: expect.objectContaining({
		// 				count: 5,
		// 			}),
		// 		});
		// 	});
		// 	test("should update actor state with replace", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create([
		// 			"test-state-replace",
		// 		]);
		// 		const actorId = await handle.resolve();
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		// Replace the entire state
		// 		const response = await http.state.$patch({
		// 			json: {
		// 				replace: { count: 10 },
		// 			},
		// 		});
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data).toEqual({
		// 			enabled: true,
		// 			state: { count: 10 },
		// 		});
		// 	});
		// 	test("should update actor state with patch", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create([
		// 			"test-state-patch",
		// 		]);
		// 		const actorId = await handle.resolve();
		// 		// Set initial state
		// 		await handle.increment(3);
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		// Patch the state
		// 		const response = await http.state.$patch({
		// 			json: {
		// 				patch: [
		// 					{
		// 						op: "replace",
		// 						path: "/count",
		// 						value: 7,
		// 					},
		// 				],
		// 			},
		// 		});
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data).toEqual({
		// 			enabled: true,
		// 			state: expect.objectContaining({
		// 				count: 7,
		// 			}),
		// 		});
		// 	});
		// 	test("should get actor connections", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create([
		// 			"test-connections",
		// 		]);
		// 		const actorId = await handle.resolve();
		// 		handle.connect();
		// 		await handle.increment(10);
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.connections.$get();
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data.connections).toEqual(
		// 			expect.arrayContaining([
		// 				expect.objectContaining({
		// 					id: expect.any(String),
		// 				}),
		// 			]),
		// 		);
		// 	});
		// 	test("should get actor events", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create(["test-events"]);
		// 		const actorId = await handle.resolve();
		// 		handle.connect();
		// 		await handle.increment(10);
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.events.$get();
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data.events).toEqual(
		// 			expect.arrayContaining([
		// 				expect.objectContaining({
		// 					type: "broadcast",
		// 					id: expect.any(String),
		// 				}),
		// 			]),
		// 		);
		// 	});
		// 	test("should clear actor events", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create([
		// 			"test-events-clear",
		// 		]);
		// 		const actorId = await handle.resolve();
		// 		handle.connect();
		// 		await handle.increment(10);
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		{
		// 			const response = await http.events.$get();
		// 			expect(response.status).toBe(200);
		// 			const data = await response.json();
		// 			expect(data.events).toEqual(
		// 				expect.arrayContaining([
		// 					expect.objectContaining({
		// 						type: "broadcast",
		// 						id: expect.any(String),
		// 					}),
		// 				]),
		// 			);
		// 		}
		// 		const response = await http.events.clear.$post();
		// 		expect(response.status).toBe(200);
		// 	});
		// 	test("should get actor rpcs", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create(["test-rpcs"]);
		// 		const actorId = await handle.resolve();
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.rpcs.$get();
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		expect(data).toEqual(
		// 			expect.objectContaining({
		// 				rpcs: expect.arrayContaining(["increment", "getCount"]),
		// 			}),
		// 		);
		// 	});
		// 	// database is not officially supported yet
		// 	test.skip("should get actor database info", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create(["test-db"]);
		// 		const actorId = await handle.resolve();
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		const response = await http.db.$get();
		// 		expect(response.status).toBe(200);
		// 		const data = await response.json();
		// 		// Database might be enabled or disabled depending on actor configuration
		// 		expect(data).toHaveProperty("enabled");
		// 		expect(typeof data.enabled).toBe("boolean");
		// 		if (data.enabled) {
		// 			expect(data).toHaveProperty("db");
		// 			expect(Array.isArray(data.db)).toBe(true);
		// 		} else {
		// 			expect(data.db).toBe(null);
		// 		}
		// 	});
		// 	test.skip("should execute database query when database is enabled", async (c) => {
		// 		const { client, endpoint } = await setupDriverTest(
		// 			c,
		// 			driverTestConfig,
		// 		);
		// 		const handle = await client.counter.create(["test-db-query"]);
		// 		const actorId = await handle.resolve();
		// 		const http = createActorInspectorClient(
		// 			`${endpoint}/actors/inspect`,
		// 			{
		// 				headers: {
		// 					Authorization: `Bearer token`,
		// 					[HEADER_ACTOR_QUERY]: JSON.stringify({
		// 						getForId: { name: "counter", actorId },
		// 					} satisfies ActorQuery),
		// 				},
		// 			},
		// 		);
		// 		// First check if database is enabled
		// 		const dbInfoResponse = await http.db.$get();
		// 		const dbInfo = await dbInfoResponse.json();
		// 		if (dbInfo.enabled) {
		// 			// Execute a simple query
		// 			const queryResponse = await http.db.$post({
		// 				json: {
		// 					query: "SELECT 1 as test",
		// 					params: [],
		// 				},
		// 			});
		// 			expect(queryResponse.status).toBe(200);
		// 			const queryData = await queryResponse.json();
		// 			expect(queryData).toHaveProperty("result");
		// 		} else {
		// 			// If database is not enabled, the POST should return enabled: false
		// 			const queryResponse = await http.db.$post({
		// 				json: {
		// 					query: "SELECT 1 as test",
		// 					params: [],
		// 				},
		// 			});
		// 			expect(queryResponse.status).toBe(200);
		// 			const queryData = await queryResponse.json();
		// 			expect(queryData).toEqual({ enabled: false });
		// 		}
		// 	});
		// });
	});
}
