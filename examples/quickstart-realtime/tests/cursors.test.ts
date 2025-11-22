import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/backend/registry";

describe("cursor room", () => {
	test("broadcasts cursor updates", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const room1 = client.cursorRoom.getOrCreate(["room1"]);

		// Update cursor from client1
		const cursor = await room1.updateCursor("user1", 100, 200);

		// Verify cursor structure
		expect(cursor).toMatchObject({
			userId: "user1",
			x: 100,
			y: 200,
		});

		// Verify getCursors returns the correct cursor
		const cursors = await room1.getCursors();
		expect(cursors).toHaveLength(1);
		expect(cursors[0]).toMatchObject({
			userId: "user1",
			x: 100,
			y: 200,
		});
	});

	test("handles multiple cursors", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const room1 = client.cursorRoom.getOrCreate(["room2"]);
		const room2 = client.cursorRoom.getOrCreate(["room2"]);
		const room3 = client.cursorRoom.getOrCreate(["room2"]);

		// Each client updates their cursor
		await room1.updateCursor("alice", 10, 20);
		await room2.updateCursor("bob", 30, 40);
		await room3.updateCursor("charlie", 50, 60);

		// Wait for all updates to propagate
		await new Promise((resolve) => setTimeout(resolve, 100));

		// Verify all clients see all three cursors
		const cursors1 = await room1.getCursors();
		const cursors2 = await room2.getCursors();
		const cursors3 = await room3.getCursors();

		expect(cursors1).toHaveLength(3);
		expect(cursors2).toHaveLength(3);
		expect(cursors3).toHaveLength(3);

		// Verify cursor positions are correct
		const aliceCursor = cursors1.find((c) => c.userId === "alice");
		const bobCursor = cursors1.find((c) => c.userId === "bob");
		const charlieCursor = cursors1.find((c) => c.userId === "charlie");

		expect(aliceCursor).toMatchObject({ userId: "alice", x: 10, y: 20 });
		expect(bobCursor).toMatchObject({ userId: "bob", x: 30, y: 40 });
		expect(charlieCursor).toMatchObject({
			userId: "charlie",
			x: 50,
			y: 60,
		});
	});

	test("multiple cursors in same room", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const room1 = client.cursorRoom.getOrCreate(["room3"]);
		const room2 = client.cursorRoom.getOrCreate(["room3"]);

		// Both clients update their cursors
		await room1.updateCursor("user1", 100, 100);
		await room2.updateCursor("user2", 200, 200);

		// Verify both cursors are present
		const cursors = await room1.getCursors();
		expect(cursors).toHaveLength(2);

		// Verify cursor data
		const user1Cursor = cursors.find((c) => c.userId === "user1");
		const user2Cursor = cursors.find((c) => c.userId === "user2");

		expect(user1Cursor).toMatchObject({ userId: "user1", x: 100, y: 100 });
		expect(user2Cursor).toMatchObject({ userId: "user2", x: 200, y: 200 });
	});
});
