import { RivetClient } from "@rivet-gg/api";
import WebSocket from "ws";

// Can be opt since they're not required for dev
const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT;
const RIVET_SERVICE_TOKEN = process.env.RIVET_SERVICE_TOKEN;
const RIVET_PROJECT = process.env.RIVET_PROJECT;
const RIVET_ENVIRONMENT = process.env.RIVET_ENVIRONMENT;

// Determine test kind from environment variable
const BUILD_NAME = process.env.BUILD;
if (BUILD_NAME !== "ws-isolate" && BUILD_NAME !== "ws-container") {
	throw new Error("Must specify BUILD environment variable as either 'ws-isolate' or 'ws-container'");
}

let region = process.env.REGION;
if (!region || region.length === 0) {
	region = undefined;
}

const client = new RivetClient({
	environment: RIVET_ENDPOINT,
	token: RIVET_SERVICE_TOKEN,
});

async function run() {
	let actorId: string | undefined;
	try {
		console.log("Creating actor", { region });
		const { actor } = await client.actor.create({
			project: RIVET_PROJECT,
			environment: RIVET_ENVIRONMENT,
			body: {
				region,
				tags: {
					name: BUILD_NAME,
				},
				buildTags: { name: BUILD_NAME, current: "true" },
				network: {
					ports: {
						http: {
							protocol: "https",
							// internalPort: 80,
							routing: {
								guard: {},
							},
						},
					},
				},
				lifecycle: {
					durable: true,
				},
				...(BUILD_NAME === "ws-container" ? {
					resources: {
						cpu: 100,
						memory: 100,
					}
				} : {}),
			},
		});
		actorId = actor.id;

		const port = actor.network.ports.http;

		const actorOrigin = `${port.protocol}://${port.hostname}:${port.port}${port.path ?? ""}`;
		console.log("Created actor at", actorOrigin);

		//await new Promise((resolve) => setTimeout(resolve, 300));

		//console.time(`ready-${actorId}`);
		//while (true) {
		//	// Check HTTP health of service
		//	const response = await fetch(`${actorOrigin}/health`);
		//	if (!response.ok) {
		//		await new Promise(resolve => setTimeout(resolve, 100));
		//		continue;
		//	}
		//	console.timeEnd(`ready-${actorId}`);
		//	break;
		//}

		// Retry loop for HTTP health check
		console.time(`ready-${actorId}`);
		while (true) {
			try {
				const response = await fetch(`${actorOrigin}/health`);
				if (response.ok) {
					console.log("Health check passed");
					console.timeEnd(`ready-${actorId}`);
					break;
				} else {
					console.error(
						`Health check failed with status: ${response.status}, retrying...`,
					);
				}
			} catch (error) {
				console.error("Health check request error:", error);
			}
			await new Promise((resolve) => setTimeout(resolve, 100));
		}

		await new Promise((resolve, reject) => {
			// Open a WebSocket to that endpoint
			const ws = new WebSocket(`${actorOrigin}/ws`);

			ws.onmessage = (evt) => {
				const [type, body] = JSON.parse(evt.data as any);
				if (type === "init") {
					console.log("Init event data:", body);
				} else if (type === "pong") {
					console.log("Pong");
					ws.close();
					resolve(undefined);
				} else {
					console.warn("unknown message type", type);
				}
			};

			ws.onopen = () => {
				console.log("Ping");
				ws.send(JSON.stringify(["ping", 123]));
			};

			ws.onclose = () => {
				console.log("WebSocket connection closed");
			};

			ws.onerror = (err) => {
				console.error("WS error", err);
				reject("ws error");
			};
		});

		console.log("Sleeping forever so you can debug");
		await new Promise(resolve => setTimeout(resolve, 100_000_000));
	} catch (error) {
		console.error("Error:", error);
	} finally {
		if (actorId) {
			console.log("Destroying", actorId);
			await client.actor.destroy(actorId, {
				project: RIVET_PROJECT,
				environment: RIVET_ENVIRONMENT,
			});
		}
	}
}

async function runLoop() {
	while (true) {
		await run();
		await new Promise((resolve) =>
			setTimeout(resolve, Math.random() * 250),
		);
	}
}

// Run loop without top-level await
(async function () {
	for (let i = 0; i < 1; i++) {
		await new Promise((resolve) => setTimeout(resolve, 100));
		runLoop();
	}
})();
