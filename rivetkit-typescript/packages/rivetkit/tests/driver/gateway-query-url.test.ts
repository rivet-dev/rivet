import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test } from "vitest";
import { setupDriverTest } from "./shared-utils";

function buildGatewayInspectorUrl(gatewayUrl: string, path: string): URL {
	const url = new URL(gatewayUrl);
	url.pathname = `${url.pathname.replace(/\/$/, "")}${path}`;
	return url;
}

describeDriverMatrix("Gateway Query Url", (driverTestConfig) => {
	describe("Gateway Query URLs", () => {
		test(
			"getOrCreate gateway URLs use rvt-* query params and resolve through the gateway",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.counter.getOrCreate(["gateway-query"]);

				await handle.increment(5);

				const gatewayUrl = await handle.getGatewayUrl();
				const parsedUrl = new URL(gatewayUrl);
				expect(
					parsedUrl.searchParams.get("rvt-namespace"),
				).toBeTruthy();
				expect(parsedUrl.searchParams.get("rvt-method")).toBe(
					"getOrCreate",
				);
				expect(parsedUrl.searchParams.get("rvt-crash-policy")).toBe(
					"sleep",
				);

				const response = await fetch(
					buildGatewayInspectorUrl(gatewayUrl, "/inspector/state"),
					{
						headers: { Authorization: "Bearer token" },
					},
				);

				expect(response.status).toBe(200);
				await expect(response.json()).resolves.toEqual({
					state: { count: 5 },
					isStateEnabled: true,
				});
			},
		);

		test(
			"get gateway URLs use rvt-* query params and resolve through the gateway",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const createHandle = client.counter.getOrCreate([
					"existing-gateway-query",
				]);
				await createHandle.increment(2);

				const gatewayUrl = await client.counter
					.get(["existing-gateway-query"])
					.getGatewayUrl();
				const parsedUrl = new URL(gatewayUrl);
				expect(
					parsedUrl.searchParams.get("rvt-namespace"),
				).toBeTruthy();
				expect(parsedUrl.searchParams.get("rvt-method")).toBe("get");

				const response = await fetch(
					buildGatewayInspectorUrl(gatewayUrl, "/inspector/state"),
					{
						headers: { Authorization: "Bearer token" },
					},
				);

				expect(response.status).toBe(200);
				await expect(response.json()).resolves.toEqual({
					state: { count: 2 },
					isStateEnabled: true,
				});
			},
		);
	});
});
