import { describe, expect, test } from "vitest";
import {
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD,
} from "@/driver-helpers/mod";
import { importWebSocket } from "@/common/websocket";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

function buildGatewayWebSocketUrl(gatewayUrl: string, path = ""): string {
	const url = new URL(gatewayUrl);
	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
	let pathPortion = path;
	let queryPortion = "";
	const queryIndex = path.indexOf("?");
	if (queryIndex !== -1) {
		pathPortion = path.slice(0, queryIndex);
		queryPortion = path.slice(queryIndex);
	}
	const normalizedPath = pathPortion.replace(/^\//, "");
	url.pathname = `${url.pathname.replace(/\/$/, "")}/websocket/${normalizedPath}`;
	if (queryPortion) {
		const extraSearchParams = new URLSearchParams(queryPortion);
		for (const [key, value] of extraSearchParams.entries()) {
			url.searchParams.append(key, value);
		}
	}
	return url.toString();
}

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) {
		return;
	}

	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener("error", reject, { once: true });
		ws.addEventListener("close", reject, { once: true });
	});
}

async function waitForJsonMessage(ws: WebSocket): Promise<Record<string, unknown>> {
	return await new Promise<Record<string, unknown>>((resolve, reject) => {
		ws.addEventListener(
			"message",
			(event: MessageEvent) => {
				try {
					resolve(JSON.parse(event.data as string));
				} catch (error) {
					reject(error);
				}
			},
			{ once: true },
		);
		ws.addEventListener("error", reject, { once: true });
		ws.addEventListener("close", reject, { once: true });
	});
}

export function runRawWebSocketDirectRegistryTests(
	driverTestConfig: DriverTestConfig,
) {
	describe("raw websocket - gateway query urls", () => {
		const httpOnlyTest =
			driverTestConfig.clientType === "http" ? test : test.skip;

		httpOnlyTest("establishes a gateway websocket connection", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const WebSocket = await importWebSocket();
			const handle = client.rawWebSocketActor.getOrCreate([
				"gateway-basic",
			]);

			const ws = new WebSocket(buildGatewayWebSocketUrl(await handle.getGatewayUrl()), [
				WS_PROTOCOL_STANDARD,
				`${WS_PROTOCOL_ENCODING}bare`,
			]) as WebSocket;

			await waitForOpen(ws);
			await expect(waitForJsonMessage(ws)).resolves.toEqual({
				type: "welcome",
				connectionCount: 1,
			});

			ws.close();
		});

		httpOnlyTest("echoes messages over gateway websocket urls", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const WebSocket = await importWebSocket();
			const handle = client.rawWebSocketActor.getOrCreate([
				"gateway-echo",
			]);

			const ws = new WebSocket(buildGatewayWebSocketUrl(await handle.getGatewayUrl()), [
				WS_PROTOCOL_STANDARD,
				`${WS_PROTOCOL_ENCODING}bare`,
			]) as WebSocket;

			await waitForOpen(ws);
			await waitForJsonMessage(ws);

			const payload = { test: "gateway", timestamp: Date.now() };
			ws.send(JSON.stringify(payload));
			await expect(waitForJsonMessage(ws)).resolves.toEqual(payload);

			ws.close();
		});

		httpOnlyTest(
			"accepts connection params over gateway websocket urls",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const WebSocket = await importWebSocket();
				const handle = client.rawWebSocketActor.getOrCreate([
					"gateway-auth",
				]);

				const ws = new WebSocket(
					buildGatewayWebSocketUrl(await handle.getGatewayUrl()),
					[
						WS_PROTOCOL_STANDARD,
						`${WS_PROTOCOL_ENCODING}bare`,
						`${WS_PROTOCOL_CONN_PARAMS}${encodeURIComponent(
							JSON.stringify({
								token: "ws-auth-token",
								userId: "ws-user123",
							}),
						)}`,
					],
				) as WebSocket;

				await waitForOpen(ws);
				await expect(waitForJsonMessage(ws)).resolves.toEqual({
					type: "welcome",
					connectionCount: 1,
				});

				ws.close();
			},
		);

		httpOnlyTest(
			"allows custom user protocols alongside rivet protocols",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const WebSocket = await importWebSocket();
				const handle = client.rawWebSocketActor.getOrCreate([
					"gateway-protocols",
				]);

				const ws = new WebSocket(
					buildGatewayWebSocketUrl(await handle.getGatewayUrl()),
					[
						WS_PROTOCOL_STANDARD,
						`${WS_PROTOCOL_ENCODING}bare`,
						"chat-v1",
						"custom-protocol",
					],
				) as WebSocket;

				await waitForOpen(ws);
				await expect(waitForJsonMessage(ws)).resolves.toEqual({
					type: "welcome",
					connectionCount: 1,
				});

				ws.close();
			},
		);

		httpOnlyTest(
			"supports custom websocket subpaths via gateway query urls",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const WebSocket = await importWebSocket();
				const handle = client.rawWebSocketActor.getOrCreate([
					"gateway-paths",
				]);

				const ws = new WebSocket(
					buildGatewayWebSocketUrl(
						await handle.getGatewayUrl(),
						"custom/path?token=secret&session=123",
					),
					[
						WS_PROTOCOL_STANDARD,
						`${WS_PROTOCOL_ENCODING}bare`,
					],
				) as WebSocket;

				await waitForOpen(ws);
				await waitForJsonMessage(ws);
				ws.send(JSON.stringify({ type: "getRequestInfo" }));

				await expect(waitForJsonMessage(ws)).resolves.toMatchObject({
					type: "requestInfo",
					pathname: expect.stringContaining("/websocket/custom/path"),
					search: "?token=secret&session=123",
				});

				ws.close();
			},
		);
	});
}
