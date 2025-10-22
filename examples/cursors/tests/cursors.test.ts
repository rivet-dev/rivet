import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/backend/registry";

test("Cursor room can handle cursor updates", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.cursorRoom.getOrCreate(["test-room"]);

	// Test initial state
	const initialCursors = await room.getCursors();
	expect(initialCursors).toEqual({});

	// Update cursor position
	const cursor1 = await room.updateCursor("user1", 100, 200);

	// Verify cursor structure
	expect(cursor1).toMatchObject({
		userId: "user1",
		x: 100,
		y: 200,
		timestamp: expect.any(Number),
	});

	// Update another cursor
	await room.updateCursor("user2", 300, 400);

	// Verify cursors are stored
	const cursors = await room.getCursors();
	expect(Object.keys(cursors)).toHaveLength(2);
	expect(cursors.user1).toBeDefined();
	expect(cursors.user2).toBeDefined();
	expect(cursors.user1.x).toBe(100);
	expect(cursors.user1.y).toBe(200);
	expect(cursors.user2.x).toBe(300);
	expect(cursors.user2.y).toBe(400);
});

test("Cursor room can place text labels", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.cursorRoom.getOrCreate(["test-text"]);

	// Test initial state
	const initialLabels = await room.getTextLabels();
	expect(initialLabels).toEqual([]);

	// Place text
	const label1 = await room.placeText("user1", "Hello", 50, 75);

	// Verify label structure
	expect(label1).toMatchObject({
		id: expect.any(String),
		userId: "user1",
		text: "Hello",
		x: 50,
		y: 75,
		timestamp: expect.any(Number),
	});

	// Place another text
	const label2 = await room.placeText("user2", "World", 150, 175);

	// Verify labels are stored in order
	const labels = await room.getTextLabels();
	expect(labels).toHaveLength(2);
	expect(labels[0]).toEqual(label1);
	expect(labels[1]).toEqual(label2);
});

test("Cursor room can remove cursors", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.cursorRoom.getOrCreate(["test-remove"]);

	// Add some cursors
	await room.updateCursor("user1", 100, 200);
	await room.updateCursor("user2", 300, 400);
	await room.updateCursor("user3", 500, 600);

	let cursors = await room.getCursors();
	expect(Object.keys(cursors)).toHaveLength(3);

	// Remove one cursor
	await room.removeCursor("user2");

	cursors = await room.getCursors();
	expect(Object.keys(cursors)).toHaveLength(2);
	expect(cursors.user1).toBeDefined();
	expect(cursors.user3).toBeDefined();
	expect(cursors.user2).toBeUndefined();
});

test("Cursor updates overwrite previous positions", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.cursorRoom.getOrCreate(["test-overwrite"]);

	// Update cursor multiple times
	await room.updateCursor("user1", 100, 200);
	const cursor2 = await room.updateCursor("user1", 300, 400);
	const cursor3 = await room.updateCursor("user1", 500, 600);

	const cursors = await room.getCursors();
	expect(Object.keys(cursors)).toHaveLength(1);
	expect(cursors.user1.x).toBe(500);
	expect(cursors.user1.y).toBe(600);
	expect(cursors.user1.timestamp).toBe(cursor3.timestamp);
	expect(cursor3.timestamp).toBeGreaterThanOrEqual(cursor2.timestamp);
});

test("Multiple users can place text in the same room", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.cursorRoom.getOrCreate(["test-multiuser-text"]);

	// Multiple users placing text
	await room.placeText("Alice", "Hello!", 10, 10);
	await room.placeText("Bob", "Hi there!", 50, 50);
	await room.placeText("Charlie", "Good day!", 100, 100);
	await room.placeText("Alice", "How are you?", 150, 150);

	const labels = await room.getTextLabels();
	expect(labels).toHaveLength(4);

	// Verify users
	expect(labels[0].userId).toBe("Alice");
	expect(labels[1].userId).toBe("Bob");
	expect(labels[2].userId).toBe("Charlie");
	expect(labels[3].userId).toBe("Alice");

	// Verify text content
	expect(labels[0].text).toBe("Hello!");
	expect(labels[1].text).toBe("Hi there!");
	expect(labels[2].text).toBe("Good day!");
	expect(labels[3].text).toBe("How are you?");
});
