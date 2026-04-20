import { expect, test } from "vitest";
import { describeDriverMatrix } from "./driver/shared-matrix";
import { setupDriverTest } from "./driver/shared-utils";

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
	},
	{ encodings: ["json"] },
);
