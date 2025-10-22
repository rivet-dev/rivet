import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/backend/registry";

test("Chat room can handle message sending and history", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.chatRoom.getOrCreate(["test-room"]);

	// Test initial state
	const initialHistory = await room.getHistory();
	expect(initialHistory).toEqual([]);

	// Send a message
	const message1 = await room.sendMessage("Alice", "Hello everyone!");

	// Verify message structure
	expect(message1).toMatchObject({
		sender: "Alice",
		text: "Hello everyone!",
		timestamp: expect.any(Number),
	});

	// Send another message
	const message2 = await room.sendMessage("Bob", "Hi Alice!");

	// Verify messages are stored in order
	const history = await room.getHistory();
	expect(history).toHaveLength(2);
	expect(history[0]).toEqual(message1);
	expect(history[1]).toEqual(message2);
});

test("Chat room message timestamps are sequential", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.chatRoom.getOrCreate(["test-timestamps"]);

	const message1 = await room.sendMessage("User1", "First message");
	const message2 = await room.sendMessage("User2", "Second message");
	const message3 = await room.sendMessage("User1", "Third message");

	expect(message2.timestamp).toBeGreaterThanOrEqual(message1.timestamp);
	expect(message3.timestamp).toBeGreaterThanOrEqual(message2.timestamp);

	const history = await room.getHistory();
	for (let i = 1; i < history.length; i++) {
		expect(history[i].timestamp).toBeGreaterThanOrEqual(
			history[i - 1].timestamp,
		);
	}
});

test("Chat room supports multiple users", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.chatRoom.getOrCreate(["test-multiuser"]);

	// Multiple users sending messages
	await room.sendMessage("Alice", "Hello!");
	await room.sendMessage("Bob", "Hey there!");
	await room.sendMessage("Charlie", "Good morning!");
	await room.sendMessage("Alice", "How is everyone?");

	const history = await room.getHistory();
	expect(history).toHaveLength(4);

	// Verify senders
	expect(history[0].sender).toBe("Alice");
	expect(history[1].sender).toBe("Bob");
	expect(history[2].sender).toBe("Charlie");
	expect(history[3].sender).toBe("Alice");

	// Verify message content
	expect(history[0].text).toBe("Hello!");
	expect(history[1].text).toBe("Hey there!");
	expect(history[2].text).toBe("Good morning!");
	expect(history[3].text).toBe("How is everyone?");
});

test("Chat room handles empty messages", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.chatRoom.getOrCreate(["test-empty"]);

	// Test empty message
	const emptyMessage = await room.sendMessage("User", "");
	expect(emptyMessage.text).toBe("");
	expect(emptyMessage.sender).toBe("User");
	expect(emptyMessage.timestamp).toBeGreaterThan(0);

	const history = await room.getHistory();
	expect(history).toHaveLength(1);
	expect(history[0]).toEqual(emptyMessage);
});
