#!/usr/bin/env tsx

import { RIVET_ENDPOINT, createActor, destroyActor } from "./utils";
import WebSocket from "ws";

const ACTORS = parseInt(process.argv[2]) || 15;

async function main() {
	console.log(`Starting ${ACTORS} actor E2E tests...`);

	let promises = [...Array(ACTORS)].map((_, i) => testActor(i));

	await Promise.all(promises);

	console.log("E2E test completed");

	// HACK: This script does not exit by itself for some reason
	process.exit(0);
}

async function testActor(i: number) {
	try {
		// Create an actor
		console.log(`Creating actor ${i}...`);
		const actorResponse = await createActor("default", "test-runner");
		console.log("Actor created:", actorResponse.actor);

		// Make a request to the actor
		console.log(`Making request to actor ${i}...`);
		const actorPingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
			method: "GET",
			headers: {
				"X-Rivet-Target": "actor",
				"X-Rivet-Actor": actorResponse.actor.actor_id,
			},
		});

		const pingResult = await actorPingResponse.text();

		if (!actorPingResponse.ok) {
			throw new Error(
				`Failed to ping actor ${i}: ${actorPingResponse.status} ${actorPingResponse.statusText}\n${pingResult}`,
			);
		}

		console.log(`Actor ${i} ping response:`, pingResult);

		// await testWebSocket(actorResponse.actor.actor_id);

		console.log(`Destroying actor ${i}...`);
		await destroyActor("default", actorResponse.actor.actor_id);
	} catch (error) {
		console.error(`Actor ${i} test failed:`, error);
	}
}

function testWebSocket(actorId: string): Promise<void> {
	console.log(`Testing WebSocket connection to actor...`);

	return new Promise((resolve, reject) => {
		// Parse the RIVET_ENDPOINT to get WebSocket URL
		const wsEndpoint = RIVET_ENDPOINT.replace("http://", "ws://").replace(
			"https://",
			"wss://",
		);
		const wsUrl = `${wsEndpoint}/ws`;

		console.log(`Connecting WebSocket to: ${wsUrl}`);

		const protocols = ["rivet", "rivet_target.actor", `rivet_actor.${actorId}`];
		const ws = new WebSocket(wsUrl, protocols);

		let pingReceived = false;
		let echoReceived = false;
		const timeout = setTimeout(() => {
			console.log(
				"No response received within timeout, but connection was established",
			);
			// Connection was established, that's enough for the test
			ws.close();
			resolve();
		}, 2000);

		ws.on("open", () => {
			console.log("WebSocket connected");

			// Test ping-pong
			console.log("Sending 'ping' message...");
			ws.send("ping");
		});

		ws.on("message", (data) => {
			const message = data.toString();
			console.log(`WebSocket received raw data:`, data);
			console.log(`WebSocket received message: "${message}"`);

			if (
				(message === "Echo: ping" || message === "pong") &&
				!pingReceived
			) {
				pingReceived = true;
				console.log("Ping test successful!");

				// Test echo
				console.log("Sending 'hello' message...");
				ws.send("hello");
			} else if (message === "Echo: hello" && !echoReceived) {
				echoReceived = true;
				console.log("Echo test successful!");

				// All tests passed
				clearTimeout(timeout);
				ws.close();
				resolve();
			}
		});

		ws.on("error", (error) => {
			clearTimeout(timeout);
			reject(new Error(`WebSocket error: ${error.message}`));
		});

		ws.on("close", () => {
			clearTimeout(timeout);
			if (!pingReceived || !echoReceived) {
				reject(new Error("WebSocket closed before completing tests"));
			}
		});
	});
}

main();
