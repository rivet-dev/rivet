import { check, sleep } from "k6";
import http from "k6/http";
import ws from "k6/ws";
import { callRivetApi } from "./rivet_api.ts";
import type { Config, CreateActorResponse } from "./types.ts";

export function createActor(config: Config): CreateActorResponse {
	return callRivetApi(config, "POST", "/actors", {
		region: config.region,
		tags: { name: "ws" },
		build_tags: { name: "ws", current: "true" },
		network: {
			ports: {
				http: {
					protocol: "https",
					routing: { guard: {} },
				},
			},
		},
		lifecycle: { durable: false },
	});
}

export function destroyActor(config: Config, actorId: string): void {
	callRivetApi(config, "DELETE", `/actors/${actorId}`);
}

export function waitForHealth(url: string): boolean {
	let attempts = 0;
	const maxAttempts = 30;

	while (attempts < maxAttempts) {
		const response = http.get(url);
		if (response.status === 200) {
			console.debug("health check passed");
			return true;
		}
		sleep(0.5);
		attempts++;
	}
	return false;
}

export function testWebSocket(url: string): void {
	sleep(0.5);

	let didOpen = false;
	let didError = false;
	let didPong = false;
	console.log("connecting to", url);
	const wsResponse = ws.connect(url, null, (socket) => {
		socket.on("open", () => {
			didOpen = true;
			console.debug("socket open, sending ping");
			socket.send(JSON.stringify(["ping", 123]));
		});

		socket.on("message", (data) => {
			const [type, _body] = JSON.parse(data);
			if (type === "pong") {
				console.debug("socket pong");
				didPong = true;
				socket.close();
			}
		});

		socket.on("error", () => {
			didError = true;
		});

		socket.setTimeout(() => {
			console.log("2 seconds passed, closing the socket from timeout");
		}, 2000);
	});

	check(wsResponse, {
		"websocket connected successfully": (r) => r && r.status === 101,
	});

	check(didOpen, { "socket opened": (x) => x });
	check(didPong, { "socket received pong": (x) => x });
	check(didError, { "socket did not error": (x) => !x });
}
