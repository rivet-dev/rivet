import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/actors.ts";

test("Cursor room can be created and initialized", async (ctx: any) => {
	const { client } = await setupTest(ctx, registry);
	const room = client.cursorRoom.getOrCreate(["test-room"]);

	// Test that the getOrCreate action works
	const result = await room.getOrCreate();
	expect(result).toEqual({ status: "ok" });
});
