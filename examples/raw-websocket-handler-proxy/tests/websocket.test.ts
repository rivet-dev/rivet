import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("websocket chat", () => {
	test("should connect and receive init message", async (test) => {
		const { client } = await setupTest(test, registry);
		const actor = client.chatRoom.getOrCreate("test-room");

		const ws = await actor.websocket();

		// Wait for init message
		const initMessage = await new Promise<any>((resolve) => {
			ws.addEventListener(
				"message",
				(event: any) => {
					resolve(JSON.parse(event.data));
				},
				{ once: true },
			);
		});

		expect(initMessage.type).toBe("init");
		expect(initMessage.messages).toEqual([]);

		ws.close();
	});

	test("should broadcast messages", async (test) => {
		const { client } = await setupTest(test, registry);
		const actor = client.chatRoom.getOrCreate("test-room-2");

		// Connect two clients
		const ws1 = await actor.websocket();
		const ws2 = await actor.websocket();

		// Skip init messages
		await new Promise((resolve) => {
			ws1.addEventListener("message", resolve, { once: true });
		});
		await new Promise((resolve) => {
			ws2.addEventListener("message", resolve, { once: true });
		});

		// Send message from client 1
		ws1.send(
			JSON.stringify({
				type: "message",
				text: "Hello from client 1",
			}),
		);

		// Both clients should receive it
		const [msg1, msg2] = await Promise.all([
			new Promise<any>((resolve) => {
				ws1.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data));
					},
					{ once: true },
				);
			}),
			new Promise<any>((resolve) => {
				ws2.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data));
					},
					{ once: true },
				);
			}),
		]);

		expect(msg1.type).toBe("message");
		expect(msg1.text).toBe("Hello from client 1");
		expect(msg2).toEqual(msg1);

		ws1.close();
		ws2.close();
	});

	test("should persist messages", async (test) => {
		const { client } = await setupTest(test, registry);
		const actor = client.chatRoom.getOrCreate("test-room-3");

		// First client sends a message
		const ws1 = await actor.websocket();
		await new Promise((resolve) => {
			ws1.addEventListener("message", resolve, { once: true });
		});

		ws1.send(
			JSON.stringify({
				type: "message",
				text: "Persistent message",
			}),
		);

		// Wait for the message to be processed
		await new Promise((resolve) => {
			ws1.addEventListener("message", resolve, { once: true });
		});
		ws1.close();

		// Second client connects and should see the message
		const ws2 = await actor.websocket();
		const initMessage = await new Promise<any>((resolve) => {
			ws2.addEventListener(
				"message",
				(event: any) => {
					resolve(JSON.parse(event.data));
				},
				{ once: true },
			);
		});

		expect(initMessage.type).toBe("init");
		expect(initMessage.messages).toHaveLength(1);
		expect(initMessage.messages[0].text).toBe("Persistent message");

		ws2.close();
	});
});
