import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("basic websocket test", () => {
	test("should handle basic websocket connection", async (t) => {
		const { client } = await setupTest(t, registry);
		const actor = client.chatRoom.getOrCreate("test-room");

		// Connect with simple path
		const ws = await actor.websocket();

		// Wait for welcome/init message
		const initMessage = await new Promise<any>((resolve, reject) => {
			ws.addEventListener(
				"message",
				(event: any) => {
					resolve(JSON.parse(event.data));
				},
				{ once: true },
			);
			ws.addEventListener("error", reject);
			ws.addEventListener("close", () =>
				reject(new Error("Connection closed")),
			);
		});

		expect(initMessage.type).toBe("init");
		expect(initMessage.messages).toEqual([]);
		expect(initMessage.users).toBeDefined();

		// Send a message
		ws.send(JSON.stringify({ type: "message", text: "Hello!" }));

		// Receive the broadcast
		const message = await new Promise<any>((resolve, reject) => {
			ws.addEventListener(
				"message",
				(event: any) => {
					resolve(JSON.parse(event.data));
				},
				{ once: true },
			);
			ws.addEventListener("error", reject);
		});

		expect(message.type).toBe("message");
		expect(message.text).toBe("Hello!");

		ws.close();
	}, 10000);
});
