import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("chat room state", () => {
	test("send and receive messages", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create two clients connected to the same room
		const client1 = client.chatRoom.getOrCreate(["room1"]);
		const client2 = client.chatRoom.getOrCreate(["room1"]);

		// Client 1 sends a message
		const sentMessage = await client1.sendMessage("Alice", "Hello!");

		// Verify message structure
		expect(sentMessage).toMatchObject({
			id: expect.any(String),
			sender: "Alice",
			text: "Hello!",
			timestamp: expect.any(Number),
		});

		// Verify getMessages includes the new message
		const messages = await client2.getMessages();
		expect(messages).toHaveLength(1);
		expect(messages[0]).toEqual(sentMessage);
	});

	test("message persistence", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Send multiple messages
		const room1 = client.chatRoom.getOrCreate(["persistent-room"]);
		await room1.sendMessage("Alice", "Message 1");
		await room1.sendMessage("Bob", "Message 2");
		await room1.sendMessage("Charlie", "Message 3");

		// Get messages from a different client instance for the same room
		const room2 = client.chatRoom.getOrCreate(["persistent-room"]);
		const messages = await room2.getMessages();

		// Verify all previously sent messages are still there
		expect(messages).toHaveLength(3);
		expect(messages[0].sender).toBe("Alice");
		expect(messages[0].text).toBe("Message 1");
		expect(messages[1].sender).toBe("Bob");
		expect(messages[1].text).toBe("Message 2");
		expect(messages[2].sender).toBe("Charlie");
		expect(messages[2].text).toBe("Message 3");
	});

	test("message ordering", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const room = client.chatRoom.getOrCreate(["ordering-room"]);

		// Send 5 messages in sequence
		const msg1 = await room.sendMessage("User", "Message 1");
		const msg2 = await room.sendMessage("User", "Message 2");
		const msg3 = await room.sendMessage("User", "Message 3");
		const msg4 = await room.sendMessage("User", "Message 4");
		const msg5 = await room.sendMessage("User", "Message 5");

		// Verify messages are returned in the correct order
		const messages = await room.getMessages();
		expect(messages).toHaveLength(5);
		expect(messages[0].text).toBe("Message 1");
		expect(messages[1].text).toBe("Message 2");
		expect(messages[2].text).toBe("Message 3");
		expect(messages[3].text).toBe("Message 4");
		expect(messages[4].text).toBe("Message 5");

		// Verify timestamps are sequential
		expect(msg2.timestamp).toBeGreaterThanOrEqual(msg1.timestamp);
		expect(msg3.timestamp).toBeGreaterThanOrEqual(msg2.timestamp);
		expect(msg4.timestamp).toBeGreaterThanOrEqual(msg3.timestamp);
		expect(msg5.timestamp).toBeGreaterThanOrEqual(msg4.timestamp);
	});

	test("clear messages", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Send several messages
		const room = client.chatRoom.getOrCreate(["clear-room"]);
		await room.sendMessage("Alice", "Message 1");
		await room.sendMessage("Bob", "Message 2");
		await room.sendMessage("Charlie", "Message 3");

		// Verify messages exist
		let messages = await room.getMessages();
		expect(messages).toHaveLength(3);

		// Call clearMessages
		const result = await room.clearMessages();
		expect(result.success).toBe(true);

		// Verify getMessages returns empty array
		messages = await room.getMessages();
		expect(messages).toHaveLength(0);
	});

	test("multiple rooms", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create clients for "room1" and "room2"
		const room1 = client.chatRoom.getOrCreate(["room1"]);
		const room2 = client.chatRoom.getOrCreate(["room2"]);

		// Send messages to room1
		await room1.sendMessage("Alice", "Room 1 message 1");
		await room1.sendMessage("Bob", "Room 1 message 2");

		// Verify room2 has no messages
		const room2Messages = await room2.getMessages();
		expect(room2Messages).toHaveLength(0);

		// Verify room1 has its messages
		const room1Messages = await room1.getMessages();
		expect(room1Messages).toHaveLength(2);

		// Send message to room2
		await room2.sendMessage("Charlie", "Room 2 message");

		// Verify messages are isolated per room
		const room1Final = await room1.getMessages();
		const room2Final = await room2.getMessages();
		expect(room1Final).toHaveLength(2);
		expect(room2Final).toHaveLength(1);
		expect(room2Final[0].text).toBe("Room 2 message");
	});
});
