import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runAccessControlTests(driverTestConfig: DriverTestConfig) {
	describe("access control", () => {
		test("actions run without entrypoint auth gating", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(["actions"]);

			const allowed = await handle.allowedAction("ok");
			expect(allowed).toBe("allowed:ok");
		});

		test("passes connection id into canPublish context", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(["publish-ctx"]);

			await handle.send("allowedQueue", { value: "one" });

			const connId = await handle.allowedGetLastCanPublishConnId();
			expect(typeof connId).toBe("string");
			expect(connId.length).toBeGreaterThan(0);
		});

		test("allows and denies queue sends, and ignores undefined queues", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(["queue"]);

			await handle.send("allowedQueue", { value: "one" });
			await expect(
				handle.send("blockedQueue", { value: "two" }),
			).rejects.toMatchObject({
				code: "forbidden",
			});
			await expect(
				handle.send("missingQueue", { value: "three" }),
			).resolves.toBeUndefined();
			await expect(
				handle.send(
					"missingQueue",
					{ value: "four" },
					{ wait: true, timeout: 50 },
				),
			).resolves.toMatchObject({ status: "completed" });

			const allowedMessage = await handle.allowedReceiveQueue();
			expect(allowedMessage).toEqual({ value: "one" });

			const remainingMessage = await handle.allowedReceiveAnyQueue();
			expect(remainingMessage).toBeNull();
		});

		test("ignores incoming queue sends when actor has no queues config", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlNoQueuesActor.getOrCreate([
				"no-queues",
			]);

			await expect(
				handle.send("anyQueue", { value: "ignored" }),
			).resolves.toBeUndefined();
			await expect(
				handle.send(
					"anyQueue",
					{ value: "ignored-wait" },
					{ wait: true, timeout: 50 },
				),
			).resolves.toMatchObject({ status: "completed" });
			expect(await handle.readAnyQueue()).toBeNull();
		});

		test("allows and denies subscriptions with canSubscribe", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate([
				"subscription",
			]);
			const conn = handle.connect();

			const allowedEventPromise = new Promise<{ value: string }>(
				(resolve, reject) => {
					const unsubscribeError = conn.onError((error) => {
						reject(error);
					});
					const unsubscribeEvent = conn.on(
						"allowedEvent",
						(payload) => {
							unsubscribeError();
							unsubscribeEvent();
							resolve(payload as { value: string });
						},
					);
				},
			);

			await conn.allowedAction("subscribe-ready");
			await conn.allowedBroadcastAllowedEvent("hello");
			expect(await allowedEventPromise).toEqual({ value: "hello" });

			const connId = await conn.allowedGetLastCanSubscribeConnId();
			expect(typeof connId).toBe("string");
			expect(connId.length).toBeGreaterThan(0);

			await conn.dispose();

			const blockedConn = handle.connect();
			blockedConn.on("blockedEvent", () => { });
			await expect(
				blockedConn.allowedAction("blocked-subscribe-ready"),
			).rejects.toMatchObject({
				code: "forbidden",
			});
			await blockedConn.dispose();
		});

		test("broadcasts undefined events without failing subscriptions", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate([
				"undefined-event",
			]);
			const conn = handle.connect();

			const eventPromise = new Promise<{ value: string }>((resolve, reject) => {
				const unsubscribeError = conn.onError((error) => {
					reject(error);
				});
				const unsubscribeEvent = conn.on("undefinedEvent", (payload) => {
					unsubscribeError();
					unsubscribeEvent();
					resolve(payload as { value: string });
				});
			});

			await conn.allowedAction("undefined-subscribe-ready");
			await conn.allowedBroadcastUndefinedEvent("wildcard");
			expect(await eventPromise).toEqual({ value: "wildcard" });

			await conn.dispose();
		});

		test("allows and denies raw request handlers", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const allowedHandle = client.accessControlActor.getOrCreate(
				["raw-request-allow"],
				{
					params: { allowRequest: true },
				},
			);
			const deniedHandle = client.accessControlActor.getOrCreate(
				["raw-request-deny"],
				{
					params: { allowRequest: false },
				},
			);

			const allowedResponse = await allowedHandle.fetch("/status");
			expect(allowedResponse.status).toBe(200);
			expect(await allowedResponse.json()).toEqual({ ok: true });

			const deniedResponse = await deniedHandle.fetch("/status");
			expect(deniedResponse.status).toBe(403);
		});

		test("allows and denies raw websocket handlers", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const allowedHandle = client.accessControlActor.getOrCreate(
				["raw-websocket-allow"],
				{
					params: { allowWebSocket: true },
				},
			);
			const ws = await allowedHandle.webSocket();
			const welcome = await new Promise<{ type: string }>((resolve) => {
				ws.addEventListener(
					"message",
					(event: any) => {
						resolve(
							JSON.parse(event.data as string) as {
								type: string;
							},
						);
					},
					{ once: true },
				);
			});
			expect(welcome.type).toBe("welcome");
			ws.close();

			const deniedHandle = client.accessControlActor.getOrCreate(
				["raw-websocket-deny"],
				{
					params: { allowWebSocket: false },
				},
			);

			let denied = false;
			try {
				const deniedWs = await deniedHandle.webSocket();
				const closeEvent = await new Promise<any>((resolve) => {
					deniedWs.addEventListener(
						"close",
						(event: any) => {
							resolve(event);
						},
						{ once: true },
					);
				});
				expect(closeEvent.code).toBe(1011);
				denied = true;
			} catch {
				denied = true;
			}
			expect(denied).toBe(true);
		});
	});
}
