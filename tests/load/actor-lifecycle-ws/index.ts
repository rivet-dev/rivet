import { fail } from "k6";
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
	let actorId: string | undefined;
	try {
		console.log("creating actor");

		// Create actor
		const { actor } = createActor(CONFIG);
		actorId = actor.id;

		console.log(`created actor ${actorId}`);

		// Get endpoint info
		const port = actor.network.ports.http;
		const actorOrigin = port.url;

		// // Wait for health check
		// const isHealthy = waitForHealth(`${actorOrigin}/health`);
		// if (!isHealthy) fail("actor did not become healthy");

		// // Test WebSocket
		// const wsUrl = `${actorOrigin.replace("http:", "ws:").replace("https:", "wss:")}/ws`;
		// testWebSocket(wsUrl);
	} finally {
		// Cleanup
		if (actorId) {
			destroyActor(CONFIG, actorId);
		}
	}
}
