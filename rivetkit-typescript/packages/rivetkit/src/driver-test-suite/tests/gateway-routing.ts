import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runGatewayRoutingTests(driverTestConfig: DriverTestConfig) {
	describe("Gateway Routing", () => {
		const httpOnlyTest =
			driverTestConfig.clientType === "http" ? test : test.skip;

		describe("Header-Based Routing", () => {
			httpOnlyTest(
				"routes HTTP request via x-rivet-target and x-rivet-actor headers",
				async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// Create an actor and resolve its ID
					const handle = client.rawHttpActor.getOrCreate([
						"header-routing",
					]);
					await handle.fetch("api/hello");
					const actorId = await handle.resolve();

					// Make a direct request using header-based routing
					const response = await fetch(
						`${endpoint}/api/hello`,
						{
							headers: {
								"x-rivet-target": "actor",
								"x-rivet-actor": actorId,
							},
						},
					);

					expect(response.ok).toBe(true);
					const data = await response.json();
					expect(data).toEqual({ message: "Hello from actor!" });
				},
			);

			httpOnlyTest(
				"returns error when x-rivet-actor header is missing",
				async (c) => {
					const { endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					const response = await fetch(
						`${endpoint}/api/hello`,
						{
							headers: {
								"x-rivet-target": "actor",
							},
						},
					);

					expect(response.ok).toBe(false);
				},
			);
		});

		describe("Query-Based Routing (rvt-* params)", () => {
			httpOnlyTest(
				"routes via rvt-method=getOrCreate with rvt-key",
				async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// First create an actor so the namespace/runner exist
					const handle = client.rawHttpActor.getOrCreate([
						"query-routing",
					]);
					await handle.fetch("api/hello");

					// Get the gateway URL and extract the rvt params pattern
					const gatewayUrl = await handle.getGatewayUrl();
					const parsedUrl = new URL(gatewayUrl);
					const namespace =
						parsedUrl.searchParams.get("rvt-namespace")!;
					const runner = parsedUrl.searchParams.get("rvt-runner")!;

					// Build a manual query-routed URL
					const queryUrl = new URL(
						`${endpoint}/gateway/rawHttpActor/api/hello`,
					);
					queryUrl.searchParams.set("rvt-namespace", namespace);
					queryUrl.searchParams.set("rvt-method", "getOrCreate");
					queryUrl.searchParams.set("rvt-key", "query-routing");
					queryUrl.searchParams.set("rvt-runner", runner);

					const response = await fetch(queryUrl.toString());
					expect(response.ok).toBe(true);
					const data = await response.json();
					expect(data).toEqual({ message: "Hello from actor!" });
				},
			);

			httpOnlyTest(
				"routes via rvt-method=get with rvt-key",
				async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// Create actor first
					const handle = client.rawHttpActor.getOrCreate([
						"query-get-routing",
					]);
					await handle.fetch("api/hello");

					const gatewayUrl = await handle.getGatewayUrl();
					const parsedUrl = new URL(gatewayUrl);
					const namespace =
						parsedUrl.searchParams.get("rvt-namespace")!;

					// Build a get-only query URL
					const queryUrl = new URL(
						`${endpoint}/gateway/rawHttpActor/api/hello`,
					);
					queryUrl.searchParams.set("rvt-namespace", namespace);
					queryUrl.searchParams.set("rvt-method", "get");
					queryUrl.searchParams.set("rvt-key", "query-get-routing");

					const response = await fetch(queryUrl.toString());
					expect(response.ok).toBe(true);
					const data = await response.json();
					expect(data).toEqual({ message: "Hello from actor!" });
				},
			);

			httpOnlyTest(
				"rejects unknown rvt-* params",
				async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					const handle = client.rawHttpActor.getOrCreate([
						"query-unknown-param",
					]);
					await handle.fetch("api/hello");

					const gatewayUrl = await handle.getGatewayUrl();
					const parsedUrl = new URL(gatewayUrl);
					const namespace =
						parsedUrl.searchParams.get("rvt-namespace")!;
					const runner = parsedUrl.searchParams.get("rvt-runner")!;

					const queryUrl = new URL(
						`${endpoint}/gateway/rawHttpActor/api/hello`,
					);
					queryUrl.searchParams.set("rvt-namespace", namespace);
					queryUrl.searchParams.set("rvt-method", "getOrCreate");
					queryUrl.searchParams.set("rvt-key", "query-unknown-param");
					queryUrl.searchParams.set("rvt-runner", runner);
					queryUrl.searchParams.set("rvt-bogus", "invalid");

					const response = await fetch(queryUrl.toString());
					expect(response.ok).toBe(false);
				},
			);

			httpOnlyTest(
				"rejects duplicate scalar rvt-* params",
				async (c) => {
					const { endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// Manually build URL with duplicate rvt-namespace
					const url = `${endpoint}/gateway/rawHttpActor/api/hello?rvt-namespace=a&rvt-namespace=b&rvt-method=get&rvt-key=dup`;

					const response = await fetch(url);
					expect(response.ok).toBe(false);
				},
			);

			httpOnlyTest(
				"strips rvt-* params before forwarding to actor",
				async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// rawHttpRequestPropertiesActor echoes back the request URL
					const handle =
						client.rawHttpRequestPropertiesActor.getOrCreate([
							"rvt-strip",
						]);
					// Prime the actor
					await handle.fetch("test-path");

					const gatewayUrl = await handle.getGatewayUrl();
					const parsedUrl = new URL(gatewayUrl);
					const namespace =
						parsedUrl.searchParams.get("rvt-namespace")!;
					const runner = parsedUrl.searchParams.get("rvt-runner")!;

					// Build URL with rvt-* params and an actor query param
					const queryUrl = new URL(
						`${endpoint}/gateway/rawHttpRequestPropertiesActor/test-path`,
					);
					queryUrl.searchParams.set("rvt-namespace", namespace);
					queryUrl.searchParams.set("rvt-method", "getOrCreate");
					queryUrl.searchParams.set("rvt-key", "rvt-strip");
					queryUrl.searchParams.set("rvt-runner", runner);
					queryUrl.searchParams.set("myParam", "myValue");

					const response = await fetch(queryUrl.toString());
					expect(response.ok).toBe(true);

					const data = (await response.json()) as {
						url: string;
					};

					// The forwarded URL should contain the actor param but not rvt-* params
					expect(data.url).toContain("myParam=myValue");
					expect(data.url).not.toContain("rvt-namespace");
					expect(data.url).not.toContain("rvt-method");
					expect(data.url).not.toContain("rvt-key");
					expect(data.url).not.toContain("rvt-runner");
				},
			);

			httpOnlyTest(
				"supports multi-component keys via comma-separated rvt-key",
				async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					const handle = client.rawHttpActor.getOrCreate([
						"tenant",
						"room",
					]);
					await handle.fetch("api/hello");

					const gatewayUrl = await handle.getGatewayUrl();
					const parsedUrl = new URL(gatewayUrl);
					const namespace =
						parsedUrl.searchParams.get("rvt-namespace")!;
					const runner = parsedUrl.searchParams.get("rvt-runner")!;

					const queryUrl = new URL(
						`${endpoint}/gateway/rawHttpActor/api/hello`,
					);
					queryUrl.searchParams.set("rvt-namespace", namespace);
					queryUrl.searchParams.set("rvt-method", "getOrCreate");
					queryUrl.searchParams.set("rvt-key", "tenant,room");
					queryUrl.searchParams.set("rvt-runner", runner);

					const response = await fetch(queryUrl.toString());
					expect(response.ok).toBe(true);
					const data = await response.json();
					expect(data).toEqual({ message: "Hello from actor!" });
				},
			);
		});
	});
}
