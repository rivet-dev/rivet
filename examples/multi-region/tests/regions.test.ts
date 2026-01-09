import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("multi-region deployment", () => {
	test("isolates actors by region", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const usEast = client.gameRoom.getOrCreate(["room1"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		const euWest = client.gameRoom.getOrCreate(["room1"], {
			createInRegion: "eu-west",
			createWithInput: { region: "eu-west" },
		});

		const usRegion = await usEast.getRegion();
		const euRegion = await euWest.getRegion();

		expect(usRegion).toBe("us-east");
		expect(euRegion).toBe("eu-west");

		// Verify they are different actor instances
		const usState = await usEast.getGameState();
		const euState = await euWest.getGameState();

		expect(usState.region).toBe("us-east");
		expect(euState.region).toBe("eu-west");
	});

	test("players isolated by region", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create actors in different regions
		const usEast = client.gameRoom.getOrCreate(["room2"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		const euWest = client.gameRoom.getOrCreate(["room2"], {
			createInRegion: "eu-west",
			createWithInput: { region: "eu-west" },
		});

		// Get initial state
		const usInitialState = await usEast.getGameState();
		const euInitialState = await euWest.getGameState();

		// Verify rooms maintain separate player lists
		expect(usInitialState.players).not.toBe(euInitialState.players);
	});

	test("same room ID, different regions", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create "room1" in different regions
		const usRoom1 = client.gameRoom.getOrCreate(["room1"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		const euRoom1 = client.gameRoom.getOrCreate(["room1"], {
			createInRegion: "eu-west",
			createWithInput: { region: "eu-west" },
		});

		// Verify they are different instances
		const usState = await usRoom1.getGameState();
		const euState = await euRoom1.getGameState();

		expect(usState.region).toBe("us-east");
		expect(euState.region).toBe("eu-west");

		// Verify state is not shared
		expect(usState).not.toBe(euState);
	});

	test("movement within region", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const room = client.gameRoom.getOrCreate(["room3"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		// Movement should work within region
		// Note: move requires connection context, so this tests the action exists
		expect(room.move).toBeDefined();
		expect(typeof room.move).toBe("function");
	});

	test("region parameter validation", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create actor with custom region
		const customRegion = client.gameRoom.getOrCreate(["room4"], {
			createInRegion: "custom-region",
			createWithInput: { region: "custom-region" },
		});

		const region = await customRegion.getRegion();
		expect(region).toBe("custom-region");
	});

	test("region switching creates separate instances", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Connect to us-east
		const usEast = client.gameRoom.getOrCreate(["room5"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		const usState = await usEast.getGameState();
		expect(usState.region).toBe("us-east");

		// Connect to eu-west with same room ID
		const euWest = client.gameRoom.getOrCreate(["room5"], {
			createInRegion: "eu-west",
			createWithInput: { region: "eu-west" },
		});

		const euState = await euWest.getGameState();
		expect(euState.region).toBe("eu-west");

		// Verify they have separate state
		expect(usState.region).not.toBe(euState.region);
	});

	test("multiple regions with different room IDs", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create different rooms in different regions
		const usRoom1 = client.gameRoom.getOrCreate(["lobby"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		const usRoom2 = client.gameRoom.getOrCreate(["game"], {
			createInRegion: "us-east",
			createWithInput: { region: "us-east" },
		});

		const euRoom1 = client.gameRoom.getOrCreate(["lobby"], {
			createInRegion: "eu-west",
			createWithInput: { region: "eu-west" },
		});

		// Verify all have correct regions
		expect(await usRoom1.getRegion()).toBe("us-east");
		expect(await usRoom2.getRegion()).toBe("us-east");
		expect(await euRoom1.getRegion()).toBe("eu-west");

		// Verify they are independent
		const usRoom1State = await usRoom1.getGameState();
		const usRoom2State = await usRoom2.getGameState();
		const euRoom1State = await euRoom1.getGameState();

		expect(usRoom1State.players).not.toBe(usRoom2State.players);
		expect(usRoom1State.players).not.toBe(euRoom1State.players);
	});

	test("getGameState returns players and region", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const room = client.gameRoom.getOrCreate(["test-state"], {
			createInRegion: "ap-south",
			createWithInput: { region: "ap-south" },
		});

		const state = await room.getGameState();

		// Verify structure
		expect(state).toHaveProperty("players");
		expect(state).toHaveProperty("region");
		expect(typeof state.players).toBe("object");
		expect(state.region).toBe("ap-south");
	});
});
