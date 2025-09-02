import { fail, sleep } from "k6";
import {
	createActor,
	destroyActor,
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
		actorId = actor.actor_id;

		let created = Date.now();
		console.log(`created actor ${actorId} ${created - start}ms`);

		// Wait for health check if not disabled
		if (!CONFIG.disableHealthcheck) {
			const isHealthy = waitForHealth(`${CONFIG.rivetEndpoint}/ping`, actorId);
			if (!isHealthy) fail("actor did not become healthy");
		}

		console.log(`actor healthy ${actorId} ${Date.now() - created}ms`);

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
