import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

function buildGatewayRequestUrl(gatewayUrl: string, path: string): string {
	const url = new URL(gatewayUrl);
	const normalizedPath = path.replace(/^\//, "");
	const requestPath = normalizedPath.startsWith("request/")
		? normalizedPath
		: `request/${normalizedPath}`;
	url.pathname = `${url.pathname.replace(/\/$/, "")}/${requestPath}`;
	return url.toString();
}

export function runRawHttpDirectRegistryTests(
	driverTestConfig: DriverTestConfig,
) {
	describe("raw http - gateway query urls", () => {
		const httpOnlyTest =
			driverTestConfig.clientType === "http" ? test : test.skip;

		httpOnlyTest("handles GET requests via gateway query urls", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.rawHttpActor.getOrCreate(["gateway-get"]);

			const response = await fetch(
				buildGatewayRequestUrl(await handle.getGatewayUrl(), "api/hello"),
			);

			expect(response.ok).toBe(true);
			expect(response.status).toBe(200);
			await expect(response.json()).resolves.toEqual({
				message: "Hello from actor!",
			});
		});

		httpOnlyTest("handles POST requests via gateway query urls", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.rawHttpActor.getOrCreate(["gateway-post"]);
			const payload = { test: "gateway", number: 456 };

			const response = await fetch(
				buildGatewayRequestUrl(await handle.getGatewayUrl(), "api/echo"),
				{
					method: "POST",
					headers: {
						"Content-Type": "application/json",
					},
					body: JSON.stringify(payload),
				},
			);

			expect(response.ok).toBe(true);
			expect(response.status).toBe(200);
			await expect(response.json()).resolves.toEqual(payload);
		});

		httpOnlyTest(
			"passes custom headers through via gateway query urls",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.rawHttpActor.getOrCreate([
					"gateway-headers",
				]);

				const response = await fetch(
					buildGatewayRequestUrl(
						await handle.getGatewayUrl(),
						"api/headers",
					),
					{
						headers: {
							"X-Custom-Header": "gateway-test-value",
							"X-Another-Header": "another-gateway-value",
						},
					},
				);

				expect(response.ok).toBe(true);
				const headers = (await response.json()) as Record<string, string>;
				expect(headers["x-custom-header"]).toBe("gateway-test-value");
				expect(headers["x-another-header"]).toBe(
					"another-gateway-value",
				);
			},
		);

		httpOnlyTest(
			"returns 404 for actors without onRequest handler via gateway query urls",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.rawHttpNoHandlerActor.getOrCreate([
					"gateway-no-handler",
				]);

				const response = await fetch(
					buildGatewayRequestUrl(
						await handle.getGatewayUrl(),
						"api/anything",
					),
				);

				expect(response.ok).toBe(false);
				expect(response.status).toBe(404);
			},
		);

		httpOnlyTest(
			"handles different HTTP methods via gateway query urls",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.rawHttpActor.getOrCreate([
					"gateway-methods",
				]);
				const baseUrl = await handle.getGatewayUrl();
				const methods = ["GET", "POST", "PUT", "DELETE", "PATCH"] as const;

				for (const method of methods) {
					const response = await fetch(
						buildGatewayRequestUrl(baseUrl, "api/echo"),
						{
							method,
							headers:
								method === "POST" ||
								method === "PUT" ||
								method === "PATCH"
									? {
											"Content-Type": "application/json",
										}
									: undefined,
							body:
								method === "POST" ||
								method === "PUT" ||
								method === "PATCH"
									? JSON.stringify({ method })
									: undefined,
						},
					);

					if (method === "POST") {
						expect(response.ok).toBe(true);
						await expect(response.json()).resolves.toEqual({
							method,
						});
					} else {
						expect(response.status).toBe(404);
					}
				}
			},
		);

		httpOnlyTest(
			"handles binary data via gateway query urls",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.rawHttpActor.getOrCreate([
					"gateway-binary",
				]);
				const binaryData = new Uint8Array([1, 2, 3, 4, 5]);

				const response = await fetch(
					buildGatewayRequestUrl(await handle.getGatewayUrl(), "api/echo"),
					{
						method: "POST",
						headers: {
							"Content-Type": "application/octet-stream",
						},
						body: binaryData,
					},
				);

				expect(response.ok).toBe(true);
				expect(Array.from(new Uint8Array(await response.arrayBuffer()))).toEqual(
					[1, 2, 3, 4, 5],
				);
			},
		);
	});
}
