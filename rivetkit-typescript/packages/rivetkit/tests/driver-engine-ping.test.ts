import { importWebSocket } from "@/common/websocket";
import {
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD,
} from "@/driver-helpers/mod";
import { expect, test } from "vitest";
import { describeDriverMatrix } from "./driver/shared-matrix";
import { setupDriverTest } from "./driver/shared-utils";

function buildGatewayRequestUrl(gatewayUrl: string, path: string): string {
	const url = new URL(gatewayUrl);
	const normalizedPath = path.replace(/^\//, "");
	url.pathname = `${url.pathname.replace(/\/$/, "")}/request/${normalizedPath}`;
	return url.toString();
}

function buildGatewayWebSocketUrl(gatewayUrl: string, path = ""): string {
	const url = new URL(gatewayUrl);
	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
	const normalizedPath = path.replace(/^\//, "");
	url.pathname = `${url.pathname.replace(/\/$/, "")}/websocket/${normalizedPath}`;
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

describeDriverMatrix(
	"engine driver smoke test",
	(driverTestConfig) => {
		test("HTTP ping returns JSON response", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate(["engine-smoke-http"]);

			const response = await actor.fetch("api/hello");

			expect(response.ok).toBe(true);
			await expect(response.json()).resolves.toEqual({
				message: "Hello from actor!",
			});
		});

		test("HTTP ping works through gateway URL", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawHttpActor.getOrCreate([
				"engine-smoke-http-gateway",
			]);

			const response = await fetch(
				buildGatewayRequestUrl(await actor.getGatewayUrl(), "api/hello"),
			);

			expect(response.ok).toBe(true);
			await expect(response.json()).resolves.toEqual({
				message: "Hello from actor!",
			});
		});

		test("WebSocket echo works", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate([
				"engine-smoke-ws",
			]);
			const ws = await actor.webSocket();

			if (ws.readyState !== WebSocket.OPEN) {
				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("open", () => resolve(), {
						once: true,
					});
					ws.addEventListener("close", reject, { once: true });
				});
			}

			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("message", () => resolve(), { once: true });
				ws.addEventListener("close", reject, { once: true });
			});

			ws.send(JSON.stringify({ type: "ping" }));

			const result = await new Promise<Record<string, unknown>>(
				(resolve, reject) => {
					ws.addEventListener(
						"message",
						(event: MessageEvent<string>) => {
							resolve(JSON.parse(event.data));
						},
						{ once: true },
					);
					ws.addEventListener("close", reject, { once: true });
				},
			);

			expect(result.type).toBe("pong");
			expect(result.timestamp).toEqual(expect.any(Number));
			ws.close();
		});

		test("WebSocket echo works through gateway URL", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const WebSocketImpl = await importWebSocket();
			const actor = client.rawWebSocketActor.getOrCreate([
				"engine-smoke-ws-gateway",
			]);
			const ws = new WebSocketImpl(
				buildGatewayWebSocketUrl(await actor.getGatewayUrl()),
				[
					WS_PROTOCOL_STANDARD,
					`${WS_PROTOCOL_ENCODING}${driverTestConfig.encoding}`,
				],
			) as WebSocket;

			await waitForOpen(ws);

			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("message", () => resolve(), { once: true });
				ws.addEventListener("close", reject, { once: true });
			});

			ws.send(JSON.stringify({ type: "ping" }));

			const result = await new Promise<Record<string, unknown>>(
				(resolve, reject) => {
					ws.addEventListener(
						"message",
						(event: MessageEvent<string>) => {
							resolve(JSON.parse(event.data));
						},
						{ once: true },
					);
					ws.addEventListener("close", reject, { once: true });
				},
			);

			expect(result.type).toBe("pong");
			expect(result.timestamp).toEqual(expect.any(Number));
			ws.close();
		});
	},
	{ encodings: ["json"] },
);
