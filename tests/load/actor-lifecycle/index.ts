import { fail, sleep } from "k6";
import {
	createActor,
	destroyActor,
	testWebSocket,
	waitForHealth,
} from "./actor.ts";
import { CONFIG } from "./config.ts";

export const options = {
	scenarios: {
		ws_test: {
			executor: "ramping-vus",
			startVUs: 1,
			stages: [
				{ duration: CONFIG.rampUpDuration, target: CONFIG.vus },
				{ duration: CONFIG.duration, target: CONFIG.vus },
			],
		},
	},
	thresholds: {
		checks: ["rate>0.9"], // 90% of checks must pass
	},
};

export default function () {
	const start = Date.now();

	let actorId: string | undefined;
	try {
		let start = Date.now();
		console.log("creating actor");

		// Create actor
		const { actor } = createActor(CONFIG);
		actorId = actor.id;

		console.log(`created actor ${actorId} ${Date.now() - start}ms`);

		// Get endpoint info
		const port = actor.network.ports.http;
		const actorOrigin = port.url;

		// Wait for health check if not disabled
		if (!CONFIG.disableHealthcheck) {
			const isHealthy = waitForHealth(`${actorOrigin}/health`);
			if (!isHealthy) fail("actor did not become healthy");
		}

		// Test WebSocket if not disabled
		if (!CONFIG.disableWebsocket) {
			const wsUrl = `${actorOrigin.replace("http:", "ws:").replace("https:", "wss:")}/ws`;
			testWebSocket(wsUrl);
		}

		// Sleep if not disabled
		if (!CONFIG.disableSleep) {
			const sleepDuration = (start + 60_000 - Date.now()) / 1000;
			console.log(`sleeping for ${sleepDuration}s`);
			sleep(sleepDuration);
		}
	} finally {
		// Cleanup
		if (actorId) {
			destroyActor(CONFIG, actorId);
		}
	}

	// const sleepDuration = (start + 60_000 - Date.now()) / 1000;
	// console.log(`sleeping for ${sleepDuration}s`);
	// sleep(sleepDuration);
}
