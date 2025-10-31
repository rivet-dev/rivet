#!/usr/bin/env tsx

import {
	getOrCreateActor,
	RIVET_ENDPOINT,
	RIVET_NAMESPACE,
	RIVET_TOKEN,
} from "./utils";

async function main() {
	try {
		console.log("Starting actor E2E test...");

		// Create an actor
		console.log("Creating actor...");
		const actorResponse = await getOrCreateActor(RIVET_NAMESPACE, "test-runner", "key");
		console.log("Actor created:", actorResponse.actor);

		for (let i = 0; i < 10; i++) {
			await testWebSocket(actorResponse.actor.actor_id);

			console.log("Sleeping actor...");
			const actorSleepResponse = await fetch(`${RIVET_ENDPOINT}/sleep`, {
				method: "GET",
				headers: {
					"X-Rivet-Token": RIVET_TOKEN,
					"X-Rivet-Target": "actor",
					"X-Rivet-Actor": actorResponse.actor.actor_id,
				},
			});

			if (!actorSleepResponse.ok) {
				const sleepResult = await actorSleepResponse.text();
				throw new Error(
					`Failed to sleep actor: ${actorSleepResponse.status} ${actorSleepResponse.statusText}\n${sleepResult}`,
				);
			}

			// console.log("Waiting...");
			// await new Promise(resolve => setTimeout(resolve, 2000));
		}

		// Make a request to the actor
		console.log("Making request to actor...");
		const actorPingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
			method: "GET",
			headers: {
				"X-Rivet-Token": RIVET_TOKEN,
				"X-Rivet-Target": "actor",
				"X-Rivet-Actor": actorResponse.actor.actor_id,
			},
		});

		const pingResult = await actorPingResponse.text();

		if (!actorPingResponse.ok) {
			throw new Error(
				`Failed to ping actor: ${actorPingResponse.status} ${actorPingResponse.statusText}\n${pingResult}`,
			);
		}

		console.log("Actor ping response:", pingResult);
	} catch (error) {
		console.error(`Actor test failed:`, error);
	}
}

function testWebSocket(actorId: string): Promise<void> {
	console.log("Testing WebSocket connection to actor...");

	return new Promise((resolve, reject) => {
		// Parse the RIVET_ENDPOINT to get WebSocket URL
		const wsEndpoint = RIVET_ENDPOINT.replace("http://", "ws://").replace(
			"https://",
			"wss://",
		);
		const wsUrl = `${wsEndpoint}/ws`;

		console.log(`Connecting WebSocket to: ${wsUrl}`);

		const protocols = [
			"rivet",
			"rivet_target.actor",
			`rivet_actor.${actorId}`,
			`rivet_token.${RIVET_TOKEN}`,
		];
		const ws = new WebSocket(wsUrl, protocols);

		let pingReceived = false;
		let echoReceived = false;

		ws.addEventListener("open", () => {
			console.log("WebSocket connected");

			// Test ping-pong
			console.log("Sending 'ping' message...");
			ws.send("ping");
		});

		ws.addEventListener("message", (ev) => {
			const message = ev.data.toString();
			console.log(`WebSocket received raw data:`, ev.data);
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
				ws.close();
				resolve();
			}
		});

		ws.addEventListener("error", (event) => {
			reject(new Error(`WebSocket error: ${event}`));
		});

		ws.addEventListener("close", event => {
			if (!pingReceived || !echoReceived) {
				reject(new Error(`WebSocket closed before completing tests: ${event.code} (${event.reason}) ${new Date().toISOString()}`));
			}
		});
	});
}

main();
