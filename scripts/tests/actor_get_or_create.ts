#!/usr/bin/env tsx

import {
	getOrCreateActorById,
	listActors,
	destroyActor,
	generateRandomKey,
} from "./utils";

const COUNT = 5;

async function main() {
	try {
		console.log("Starting actor get-or-create test...");

		const namespaceName = "default";
		const actorName = "test-actor";
		const runnerNameSelector = "test-runner";

		// Generate a random key that all actor creation calls will use
		const sharedKey = generateRandomKey();
		console.log(`Using shared key: ${sharedKey}`);

		// Create parallel calls to get or create actor with the same key
		console.log(
			`Creating ${COUNT} parallel get-or-create calls with shared key...`,
		);
		let completedCount = 0;
		const promises = Array.from({ length: COUNT }, (_, index) =>
			getOrCreateActorById(
				namespaceName,
				actorName,
				sharedKey,
				runnerNameSelector,
			).then((response) => {
				completedCount++;
				console.log(
					`Call ${index + 1}/${COUNT} completed ${JSON.stringify(response)} (${completedCount} total)`,
				);
				return { index, response };
			}),
		);

		const results = await Promise.all(promises);
		console.log(`✓ Completed all ${COUNT} parallel calls`);

		// Extract all actor IDs and verify they're all the same
		const actorIds = results.map((result) => result.response.actor_id);
		const uniqueActorIds = [...new Set(actorIds)];

		if (uniqueActorIds.length !== 1) {
			console.error("Actor IDs from all calls:", actorIds);
			throw new Error(
				`Expected all calls to return the same actor ID, but got ${uniqueActorIds.length} unique IDs: ${uniqueActorIds.join(", ")}`,
			);
		}

		const firstActorId = uniqueActorIds[0];
		console.log(
			`✓ All ${COUNT} calls returned the same actor ID: ${firstActorId}`,
		);

		// List actors with the specific name to verify only one exists
		console.log("Listing actors to verify only one exists with the key...");
		const listResponse = await listActors(namespaceName, actorName);
		const actorsWithSharedKey = listResponse.actors.filter(
			(actor: any) => actor.key === sharedKey,
		);
		console.log("Actors with key:", actorsWithSharedKey);

		// Filter actors that have the shared key
		if (actorsWithSharedKey.length !== 1) {
			throw new Error(
				`Expected exactly 1 actor with key '${sharedKey}', but found ${actorsWithSharedKey.length}`,
			);
		}
		console.log(
			`✓ Found exactly 1 actor with the shared key: ${sharedKey}`,
		);

		// Verify the actor ID matches what we got from get-or-create
		const listedActor = actorsWithSharedKey[0];
		if (listedActor.actor_id !== firstActorId) {
			throw new Error(
				`Listed actor ID (${listedActor.actor_id}) doesn't match expected ID (${firstActorId})`,
			);
		}
		console.log(`✓ Listed actor ID matches expected: ${firstActorId}`);

		// Clean up: destroy the actor
		console.log("Cleaning up: destroying actor...");
		await destroyActor(namespaceName, firstActorId);

		console.log("✓ Actor get-or-create test completed successfully!");
	} catch (error) {
		console.error("❌ Actor get-or-create test failed:", error);
		process.exit(1);
	}
}

main();
