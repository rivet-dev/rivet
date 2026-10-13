import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

describeDriverMatrix("Gateway Routing", (driverTestConfig) => {
	describe("Gateway Routing", () => {
		describe("Header-Based Routing", () => {
			test("routes HTTP request via x-rivet-target and x-rivet-actor headers", async (c) => {
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

				// Make a direct request using header-based routing.
				const response = await fetch(`${endpoint}/request/api/hello`, {
					headers: {
						"x-rivet-target": "actor",
						"x-rivet-actor": actorId,
					},
				});

				expect(response.ok).toBe(true);
				const data = await response.json();
				expect(data).toEqual({ message: "Hello from actor!" });
			});

			test("does not route non-request HTTP paths to onRequest", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = client.rawHttpActor.getOrCreate([
					"header-routing-no-request",
				]);
				await handle.fetch("api/hello");
				const actorId = await handle.resolve();

				const response = await fetch(`${endpoint}/api/hello`, {
					headers: {
						"x-rivet-target": "actor",
						"x-rivet-actor": actorId,
					},
				});

				expect(response.ok).toBe(false);
			});

			test("returns error when x-rivet-actor header is missing", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				const response = await fetch(`${endpoint}/api/hello`, {
					headers: {
						"x-rivet-target": "actor",
					},
				});

				expect(response.ok).toBe(false);
			});
		});

		describe("Query-Based Routing (rvt-* params)", () => {
			test("routes via rvt-method=getOrCreate with rvt-key", async (c) => {
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
				const namespace = parsedUrl.searchParams.get("rvt-namespace")!;
				const runner = parsedUrl.searchParams.get("rvt-runner")!;

				// Build a manual query-routed URL
				const queryUrl = new URL(
					`${endpoint}/gateway/rawHttpActor/request/api/hello`,
				);
				queryUrl.searchParams.set("rvt-namespace", namespace);
				queryUrl.searchParams.set("rvt-method", "getOrCreate");
				queryUrl.searchParams.set("rvt-key", "query-routing");
				queryUrl.searchParams.set("rvt-runner", runner);

				const response = await fetch(queryUrl.toString());
				expect(response.ok).toBe(true);
				const data = await response.json();
				expect(data).toEqual({ message: "Hello from actor!" });
			});

			test("routes via rvt-method=get with rvt-key", async (c) => {
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
				const namespace = parsedUrl.searchParams.get("rvt-namespace")!;

				// Build a get-only query URL
				const queryUrl = new URL(
					`${endpoint}/gateway/rawHttpActor/request/api/hello`,
				);
				queryUrl.searchParams.set("rvt-namespace", namespace);
				queryUrl.searchParams.set("rvt-method", "get");
				queryUrl.searchParams.set("rvt-key", "query-get-routing");

				const response = await fetch(queryUrl.toString());
				expect(response.ok).toBe(true);
				const data = await response.json();
				expect(data).toEqual({ message: "Hello from actor!" });
			});

			test("rejects unknown rvt-* params", async (c) => {
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
				const namespace = parsedUrl.searchParams.get("rvt-namespace")!;
				const runner = parsedUrl.searchParams.get("rvt-runner")!;

				const queryUrl = new URL(
					`${endpoint}/gateway/rawHttpActor/request/api/hello`,
				);
				queryUrl.searchParams.set("rvt-namespace", namespace);
				queryUrl.searchParams.set("rvt-method", "getOrCreate");
				queryUrl.searchParams.set("rvt-key", "query-unknown-param");
				queryUrl.searchParams.set("rvt-runner", runner);
				queryUrl.searchParams.set("rvt-bogus", "invalid");

				const response = await fetch(queryUrl.toString());
				expect(response.ok).toBe(false);
			});

			test("rejects duplicate scalar rvt-* params", async (c) => {
				const { endpoint } = await setupDriverTest(c, driverTestConfig);

				// Manually build URL with duplicate rvt-namespace
				const url = `${endpoint}/gateway/rawHttpActor/request/api/hello?rvt-namespace=a&rvt-namespace=b&rvt-method=get&rvt-key=dup`;

				const response = await fetch(url);
				expect(response.ok).toBe(false);
			});

			test("strips rvt-* params before forwarding to actor", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// rawHttpRequestPropertiesActor echoes back the request URL
				const handle = client.rawHttpRequestPropertiesActor.getOrCreate(
					["rvt-strip"],
				);
				// Prime the actor
				await handle.fetch("test-path");

				const gatewayUrl = await handle.getGatewayUrl();
				const parsedUrl = new URL(gatewayUrl);
				const namespace = parsedUrl.searchParams.get("rvt-namespace")!;
				const runner = parsedUrl.searchParams.get("rvt-runner")!;

				// Build URL with rvt-* params and an actor query param
				const queryUrl = new URL(
					`${endpoint}/gateway/rawHttpRequestPropertiesActor/request/test-path`,
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
			});

			test("supports multi-component keys via legacy comma-separated rvt-key", async (c) => {
				// Legacy encoding, kept for backward compatibility with
				// clients that predate rvt-key-part. Cannot express a
				// component containing a literal comma; see the
				// rvt-key-part tests below for the fix.
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
				const namespace = parsedUrl.searchParams.get("rvt-namespace")!;
				const runner = parsedUrl.searchParams.get("rvt-runner")!;

				const queryUrl = new URL(
					`${endpoint}/gateway/rawHttpActor/request/api/hello`,
				);
				queryUrl.searchParams.set("rvt-namespace", namespace);
				queryUrl.searchParams.set("rvt-method", "getOrCreate");
				queryUrl.searchParams.set("rvt-key", "tenant,room");
				queryUrl.searchParams.set("rvt-runner", runner);

				const response = await fetch(queryUrl.toString());
				expect(response.ok).toBe(true);
				const data = await response.json();
				expect(data).toEqual({ message: "Hello from actor!" });
			});

			test("supports multi-component keys via repeated rvt-key-part", async (c) => {
				const { client, endpoint } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = client.rawHttpActor.getOrCreate([
					"tenant-part",
					"room-part",
				]);
				await handle.fetch("api/hello");

				const gatewayUrl = await handle.getGatewayUrl();
				const parsedUrl = new URL(gatewayUrl);
				const namespace = parsedUrl.searchParams.get("rvt-namespace")!;
				const runner = parsedUrl.searchParams.get("rvt-runner")!;
				expect(parsedUrl.searchParams.getAll("rvt-key-part")).toEqual([
					"tenant-part",
					"room-part",
				]);

				const queryUrl = new URL(
					`${endpoint}/gateway/rawHttpActor/request/api/hello`,
				);
				queryUrl.searchParams.set("rvt-namespace", namespace);
				queryUrl.searchParams.set("rvt-method", "getOrCreate");
				queryUrl.searchParams.append("rvt-key-part", "tenant-part");
				queryUrl.searchParams.append("rvt-key-part", "room-part");
				queryUrl.searchParams.set("rvt-runner", runner);

				const response = await fetch(queryUrl.toString());
				expect(response.ok).toBe(true);
				const data = await response.json();
				expect(data).toEqual({ message: "Hello from actor!" });
			});

			test("distinguishes a comma-containing component from a two-component key (issue #5807)", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// One component containing a literal comma.
				const singlePart = client.rawHttpActor.getOrCreate([
					"tenant-x,admin-x",
				]);
				await singlePart.fetch("api/hello");
				const singlePartId = await singlePart.resolve();

				// Two components, no literal comma.
				const twoParts = client.rawHttpActor.getOrCreate([
					"tenant-x",
					"admin-x",
				]);
				await twoParts.fetch("api/hello");
				const twoPartsId = await twoParts.resolve();

				expect(singlePartId).not.toBe(twoPartsId);
			});
		});
	});
});
