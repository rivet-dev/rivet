import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorInspectorTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Inspector HTTP API", () => {
		test("GET /inspector/state returns actor state", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["inspector-state"]);

			// Set some state first
			await handle.increment(5);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(`${gatewayUrl}/inspector/state`, {
				headers: { Authorization: "Bearer token" },
			});
			expect(response.status).toBe(200);
			const data = await response.json();
			expect(data).toEqual({ state: { count: 5 } });
		});

		test("PATCH /inspector/state updates actor state", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-set-state",
			]);

			await handle.increment(5);

			const gatewayUrl = await handle.getGatewayUrl();

			// Replace state
			const patchResponse = await fetch(
				`${gatewayUrl}/inspector/state`,
				{
					method: "PATCH",
					headers: {
						"Content-Type": "application/json",
						Authorization: "Bearer token",
					},
					body: JSON.stringify({ state: { count: 42 } }),
				},
			);
			expect(patchResponse.status).toBe(200);
			const patchData = await patchResponse.json();
			expect(patchData).toEqual({ ok: true });

			// Verify via action
			const count = await handle.getCount();
			expect(count).toBe(42);
		});

		test("GET /inspector/connections returns connections list", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-connections",
			]);

			// Ensure actor exists
			await handle.increment(0);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(
				`${gatewayUrl}/inspector/connections`,
				{
					headers: { Authorization: "Bearer token" },
				},
			);
			expect(response.status).toBe(200);
			const data = (await response.json()) as {
				connections: unknown[];
			};
			expect(data).toHaveProperty("connections");
			expect(Array.isArray(data.connections)).toBe(true);
		});

		test("GET /inspector/rpcs returns available actions", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["inspector-rpcs"]);

			// Ensure actor exists
			await handle.increment(0);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(`${gatewayUrl}/inspector/rpcs`, {
				headers: { Authorization: "Bearer token" },
			});
			expect(response.status).toBe(200);
			const data = (await response.json()) as { rpcs: string[] };
			expect(data).toHaveProperty("rpcs");
			expect(data.rpcs).toContain("increment");
			expect(data.rpcs).toContain("getCount");
			expect(data.rpcs).toContain("setCount");
		});

		test("POST /inspector/action/:name executes an action", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-action",
			]);

			await handle.increment(10);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(
				`${gatewayUrl}/inspector/action/increment`,
				{
					method: "POST",
					headers: {
						"Content-Type": "application/json",
						Authorization: "Bearer token",
					},
					body: JSON.stringify({ args: [5] }),
				},
			);
			expect(response.status).toBe(200);
			const data = (await response.json()) as { output: number };
			expect(data.output).toBe(15);

			// Verify via normal action
			const count = await handle.getCount();
			expect(count).toBe(15);
		});

		test("GET /inspector/queue returns queue status", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["inspector-queue"]);

			// Ensure actor exists
			await handle.increment(0);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(
				`${gatewayUrl}/inspector/queue?limit=10`,
				{
					headers: { Authorization: "Bearer token" },
				},
			);
			expect(response.status).toBe(200);
			const data = (await response.json()) as {
				size: number;
				maxSize: number;
				truncated: boolean;
				messages: unknown[];
			};
			expect(data).toHaveProperty("size");
			expect(data).toHaveProperty("maxSize");
			expect(data).toHaveProperty("truncated");
			expect(data).toHaveProperty("messages");
			expect(typeof data.size).toBe("number");
			expect(typeof data.maxSize).toBe("number");
			expect(typeof data.truncated).toBe("boolean");
			expect(Array.isArray(data.messages)).toBe(true);
		});

		test("GET /inspector/traces returns trace data", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-traces",
			]);

			// Perform an action to generate traces
			await handle.increment(1);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(
				`${gatewayUrl}/inspector/traces?startMs=0&endMs=${Date.now() + 60000}&limit=100`,
				{
					headers: { Authorization: "Bearer token" },
				},
			);
			expect(response.status).toBe(200);
			const data = (await response.json()) as {
				otlp: unknown;
				clamped: boolean;
			};
			expect(data).toHaveProperty("otlp");
			expect(data).toHaveProperty("clamped");
			expect(typeof data.clamped).toBe("boolean");
		});

		test("GET /inspector/workflow-history returns workflow status", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-workflow",
			]);

			// Ensure actor exists
			await handle.increment(0);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(
				`${gatewayUrl}/inspector/workflow-history`,
				{
					headers: { Authorization: "Bearer token" },
				},
			);
			expect(response.status).toBe(200);
			const data = (await response.json()) as {
				history: unknown;
				isWorkflowEnabled: boolean;
			};
			expect(data).toHaveProperty("history");
			expect(data).toHaveProperty("isWorkflowEnabled");
			// Counter actor has no workflow, so it should be disabled
			expect(data.isWorkflowEnabled).toBe(false);
			expect(data.history).toBeNull();
		});

		test("GET /inspector/summary returns full actor snapshot", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-summary",
			]);

			await handle.increment(7);

			const gatewayUrl = await handle.getGatewayUrl();
			const response = await fetch(
				`${gatewayUrl}/inspector/summary`,
				{
					headers: { Authorization: "Bearer token" },
				},
			);
			expect(response.status).toBe(200);
			const data = (await response.json()) as {
				state: { count: number };
				connections: unknown[];
				rpcs: string[];
				queueSize: number;
				isStateEnabled: boolean;
				isDatabaseEnabled: boolean;
				isWorkflowEnabled: boolean;
				workflowHistory: unknown;
			};
			expect(data.state).toEqual({ count: 7 });
			expect(Array.isArray(data.connections)).toBe(true);
			expect(data.rpcs).toContain("increment");
			expect(typeof data.queueSize).toBe("number");
			expect(data.isStateEnabled).toBe(true);
			expect(typeof data.isDatabaseEnabled).toBe("boolean");
			expect(data.isWorkflowEnabled).toBe(false);
			expect(data.workflowHistory).toBeNull();
		});

		test("inspector endpoints require auth in non-dev mode", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate([
				"inspector-auth",
			]);

			await handle.increment(0);

			const gatewayUrl = await handle.getGatewayUrl();

			// Request with wrong token should fail
			const response = await fetch(`${gatewayUrl}/inspector/state`, {
				headers: { Authorization: "Bearer wrong-token" },
			});
			expect(response.status).toBe(401);
		});
	});
}
