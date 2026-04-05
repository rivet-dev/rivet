import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runGatewayQueryUrlTests(driverTestConfig: DriverTestConfig) {
	describe("Gateway Query URLs", () => {
		const httpOnlyTest =
			driverTestConfig.clientType === "http" ? test : test.skip;

		httpOnlyTest(
			"getOrCreate gateway URLs stay query-backed and resolve through the gateway",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.counter.getOrCreate(["gateway-query"]);

				await handle.increment(5);

				const gatewayUrl = await handle.getGatewayUrl();
				expect(gatewayUrl).toContain(";namespace=");
				expect(gatewayUrl).toContain(";method=getOrCreate;");
				expect(gatewayUrl).toContain(";crashPolicy=sleep");

				const response = await fetch(`${gatewayUrl}/inspector/state`, {
					headers: { Authorization: "Bearer token" },
				});

				expect(response.status).toBe(200);
				await expect(response.json()).resolves.toEqual({
					state: { count: 5 },
					isStateEnabled: true,
				});
			},
		);

		httpOnlyTest(
			"get gateway URLs stay query-backed and resolve through the gateway",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const createHandle = client.counter.getOrCreate([
					"existing-gateway-query",
				]);
				await createHandle.increment(2);

				const gatewayUrl = await client.counter
					.get(["existing-gateway-query"])
					.getGatewayUrl();
				expect(gatewayUrl).toContain(";namespace=");
				expect(gatewayUrl).toContain(";method=get;");

				const response = await fetch(`${gatewayUrl}/inspector/state`, {
					headers: { Authorization: "Bearer token" },
				});

				expect(response.status).toBe(200);
				await expect(response.json()).resolves.toEqual({
					state: { count: 2 },
					isStateEnabled: true,
				});
			},
		);
	});
}
