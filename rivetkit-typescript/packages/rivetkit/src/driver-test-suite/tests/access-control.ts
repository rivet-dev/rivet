import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runAccessControlTests(driverTestConfig: DriverTestConfig) {
	describe("access control", () => {
		test("allows configured actions and denies others", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(["actions"]);

			const allowed = await handle.allowedAction("ok");
			expect(allowed).toBe("allowed:ok");

			await expect(handle.blockedAction()).rejects.toMatchObject({
				code: "forbidden",
			});
		});

		test("passes connection id into canInvoke context", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(["conn-id"]);

			const connId = await handle.allowedGetLastCanInvokeConnId();
			expect(typeof connId).toBe("string");
			expect(connId.length).toBeGreaterThan(0);
		});

		test("allows and denies queue sends", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(["queue"]);

			await handle.send("allowedQueue", { value: "one" });
			await expect(
				handle.send("blockedQueue", { value: "two" }),
			).rejects.toMatchObject({
				code: "forbidden",
			});

			const message = await handle.allowedReceiveQueue();
			expect(message).toEqual({ value: "one" });
		});

		test("allows and denies subscriptions", async (c) => {
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

			await conn.allowedBroadcastAllowedEvent("hello");
			expect(await allowedEventPromise).toEqual({ value: "hello" });

			const blockedErrorPromise = new Promise<{ code: string }>(
				(resolve) => {
					const unsubscribe = conn.onError((error) => {
						unsubscribe();
						resolve(error as { code: string });
					});
					conn.on("blockedEvent", () => { });
				},
			);

			const blockedError = await blockedErrorPromise;
			expect(blockedError.code).toBe("forbidden");

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

		test("throws when canInvoke does not return boolean", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.accessControlActor.getOrCreate(
				["invalid-return"],
				{
					params: { invalidCanInvokeReturn: true },
				},
			);

			await expect(handle.allowedAction("x")).rejects.toMatchObject({
				code: "internal_error",
			});
		});
	});
}
