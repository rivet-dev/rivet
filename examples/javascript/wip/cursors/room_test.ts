import { Client } from "@rivet-gg/actor-client";
import type { ActorHandle } from "@rivet-gg/actor-client";
import { assertEquals, assertExists } from "@std/assert";
import { readEndpointFromCli } from "../../sdks/actors/client/src/dev.ts";
import type Room from "./room.ts";
import type { BroadcastState } from "./room.ts";

async function main() {
	const actorClient = new Client(await readEndpointFromCli());
	let roomActor: ActorHandle<Room>;

	// Test connection and initial state
	{
		roomActor = await actorClient.withTags({ name: "room" });

		// Listen for state broadcasts
		let lastBroadcastState: BroadcastState | undefined = undefined;
		roomActor.on<[BroadcastState]>("state", (state) => {
			lastBroadcastState = state;
		});

		// Wait for initial state broadcast
		await new Promise((resolve) => setTimeout(resolve, 100));

		assertExists(lastBroadcastState, "Should receive initial state");
		assertEquals(
			lastBroadcastState.entities.length,
			10,
			"Should have 10 entities",
		);
		assertEquals(
			lastBroadcastState.cursors.length,
			1,
			"Should have 1 cursor for connected client",
		);
	}

	// Test cursor movement
	{
		const newX = 0.5;
		const newY = 0.7;
		await roomActor.moveCursor(newX, newY);

		// Wait for state update
		await new Promise((resolve) => setTimeout(resolve, 100));

		// Verify cursor position through next broadcast
		roomActor.on("state", (state) => {
			const myCursor = state.cursors[0];
			assertEquals(myCursor.x, newX, "Cursor X should be updated");
			assertEquals(myCursor.y, newY, "Cursor Y should be updated");
		});
	}

	// Test entity movement
	{
		const entityIndex = 0;
		const newX = 0.3;
		const newY = 0.4;
		await roomActor.moveEntity(entityIndex, newX, newY);

		// Wait for state update
		await new Promise((resolve) => setTimeout(resolve, 100));

		// Verify entity position through next broadcast
		roomActor.on("state", (state) => {
			const movedEntity = state.entities[entityIndex];
			assertEquals(movedEntity.x, newX, "Entity X should be updated");
			assertEquals(movedEntity.y, newY, "Entity Y should be updated");
		});
	}

	// Cleanup
	await roomActor.disconnect();
}

await main();
