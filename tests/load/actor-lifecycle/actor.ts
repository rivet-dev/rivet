import { check, sleep } from "k6";
import http from "k6/http";
import ws from "k6/ws";
import { callRivetApi } from "./rivet_api.ts";
import type { Config, CreateActorResponse } from "./types.ts";

export function createActor(config: Config): CreateActorResponse {
	return callRivetApi(config, "POST", "/actors", {
		name: "test-actor",
		keys: {
			idx: Math.floor(Math.random() * 10000000).toString(),
		},
		runner_name_selector: "foo",
		durable: false,
	});
}

export function destroyActor(config: Config, actorId: string): void {
	callRivetApi(config, "DELETE", `/actors/${actorId}`);
}

export function waitForHealth(url: string, actorId: string): boolean {
	let attempts = 0;
	const maxAttempts = 30;

	while (attempts < maxAttempts) {
		const response = http.get(url, {
			headers: {
				"x-rivet-target": "actor",
				"x-rivet-actor": actorId,
				"x-rivet-addr": "main",
			}
		});
		if (response.status === 200) {
			console.debug("health check passed");
			return true;
		}
		sleep(0.5);
		attempts++;
	}
	return false;
}
