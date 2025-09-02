#!/usr/bin/env tsx

import { RIVET_ENDPOINT, destroyActor } from "./utils";

// Extended version that supports datacenter selection
async function createActorInDc(
	namespaceName: string,
	runnerNameSelector: string,
	datacenter: string,
): Promise<any> {
	const response = await fetch(
		`${RIVET_ENDPOINT}/actors?namespace=${namespaceName}`,
		{
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				name: "thingy",
				key: crypto.randomUUID(),
				input: btoa("hello"),
				runner_name_selector: runnerNameSelector,
				crash_policy: "destroy",
				datacenter: datacenter, // Add datacenter to request body
			}),
		},
	);

	if (!response.ok) {
		throw new Error(
			`Failed to create actor: ${response.status} ${response.statusText}\n${await response.text()}`,
		);
	}

	return response.json();
}

async function testActorInDc(dc: string) {
	console.log(`\n=== Testing actor in ${dc} ===`);
	
	try {
		// Create an actor in the specified datacenter
		console.log(`Creating actor in ${dc}...`);
		const actorResponse = await createActorInDc(
			"default",
			"test-runner",
			dc
		);
		console.log(`Actor created in ${dc}:`, actorResponse.actor);

		// Make a request to the actor
		console.log(`Making request to actor in ${dc}...`);
		const actorPingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
			method: "GET",
			headers: {
				"x-rivet-target": "actor",
				"x-rivet-actor": actorResponse.actor.actor_id,
				"x-rivet-addr": "main",
			},
		});

		const pingResult = await actorPingResponse.text();

		if (!actorPingResponse.ok) {
			throw new Error(
				`Failed to ping actor in ${dc}: ${actorPingResponse.status} ${actorPingResponse.statusText}\n${pingResult}`,
			);
		}

		console.log(`Actor ping response from ${dc}:`, pingResult);

		console.log(`Destroying actor in ${dc}...`);
		await destroyActor("default", actorResponse.actor.actor_id);

		console.log(`✓ Test completed successfully for ${dc}!`);
		return true;
	} catch (error) {
		console.error(`✗ Test failed for ${dc}:`, error);
		return false;
	}
}

async function main() {
	const datacenters = ["dc-a", "dc-b", "dc-c"];
	const results: Record<string, boolean> = {};
	
	console.log("Starting multi-datacenter actor E2E test...");
	console.log(`Testing datacenters: ${datacenters.join(", ")}`);

	for (const dc of datacenters) {
		results[dc] = await testActorInDc(dc);
	}

	// Print summary
	console.log("\n=== Test Summary ===");
	let allPassed = true;
	for (const [dc, passed] of Object.entries(results)) {
		console.log(`${dc}: ${passed ? "✓ PASSED" : "✗ FAILED"}`);
		if (!passed) allPassed = false;
	}

	if (allPassed) {
		console.log("\nAll multi-datacenter E2E tests completed successfully!");
	} else {
		console.error("\nSome multi-datacenter E2E tests failed!");
		process.exit(1);
	}
}

main();
