import readline from "readline";
import { RivetClient } from "@rivet-gg/api";

// Read environment variables
const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT;
const RIVET_SERVICE_TOKEN = process.env.RIVET_SERVICE_TOKEN;
const RIVET_PROJECT = process.env.RIVET_PROJECT;
const RIVET_ENVIRONMENT = process.env.RIVET_ENVIRONMENT;
const BETTERSTACK_TOKEN = process.env.BETTERSTACK_TOKEN;
const BETTERSTACK_HOST = process.env.BETTERSTACK_HOST;

// Check required environment variables
if (!RIVET_SERVICE_TOKEN) {
	throw new Error("RIVET_SERVICE_TOKEN environment variable is required");
}
if (!RIVET_PROJECT) {
	throw new Error("RIVET_PROJECT environment variable is required");
}
if (!RIVET_ENVIRONMENT) {
	throw new Error("RIVET_ENVIRONMENT environment variable is required");
}
if (!BETTERSTACK_TOKEN) {
	throw new Error("BETTERSTACK_TOKEN environment variable is required");
}
if (!BETTERSTACK_HOST) {
	throw new Error("BETTERSTACK_HOST environment variable is required");
}

const client = new RivetClient({
	environment: RIVET_ENDPOINT,
	token: RIVET_SERVICE_TOKEN,
});

let actorId;

// Function to clean up and destroy the actor
async function cleanupActor() {
	if (actorId) {
		console.log("\nDestroying actor", actorId);
		try {
			await client.actor.destroy(actorId, {
				project: RIVET_PROJECT,
				environment: RIVET_ENVIRONMENT,
			});
			console.log("Actor destroyed successfully");
		} catch (error) {
			console.error("Error destroying actor:", error);
		}
		process.exit(0);
	}
}

// Handle Ctrl+C
process.on("SIGINT", async () => {
	console.log("\nCtrl+C detected, cleaning up...");
	await cleanupActor();
});

// Generate lobby URL for logs viewing
function generatelobbyUrl(lobbyId) {
	const encodedProjectId = encodeURIComponent(RIVET_PROJECT);
	const encodedEnvironmentId = encodeURIComponent(RIVET_ENVIRONMENT);

	// Encode the tags parameter properly - this is a JSON structure that needs to be encoded
	const tagsParam = encodeURIComponent(JSON.stringify([["lobby", lobbyId]]));

	return `https://hub.rivet.gg/projects/${encodedProjectId}/environments/${encodedEnvironmentId}/actors?tab=logs&showDestroyed=true&tags=${tagsParam}`;
}

async function run() {
	try {
		const lobbyId = crypto.randomUUID();
		const lobbyUrl = generatelobbyUrl(lobbyId);
		console.log("Creating actor", { lobbyId, lobbyUrl });
		const { actor } = await client.actor.create({
			project: RIVET_PROJECT,
			environment: RIVET_ENVIRONMENT,
			body: {
				tags: {
					name: "example",
					lobby: lobbyId,
				},
				buildTags: { name: "example", current: "true" },
				network: {
					ports: {
						http: {
							protocol: "https",
							routing: {
								guard: {},
							},
						},
					},
				},
				resources: {
					cpu: 100,
					memory: 100,
				},
				// Pass BetterStack credentials as environment variables
				env: {
					LOBBY_ID: lobbyId,
					LOBBY_URL: lobbyUrl,
					BETTERSTACK_TOKEN,
					BETTERSTACK_HOST,
				},
				lifecycle: {
					durable: false,
				},
			},
		});
		actorId = actor.id;

		const port = actor.network.ports.http;
		if (!port) throw new Error("missing port http");
		console.log("Created actor at", port.url);

		// Generate and display the lobby URL for viewing logs
		console.log("\nView logs at:", lobbyUrl);

		// Setup readline interface for user input
		const rl = readline.createInterface({
			input: process.stdin,
			output: process.stdout,
		});

		// Wait for user to press Enter
		await new Promise((resolve) => {
			rl.question("Press Enter to destroy the actor...", () => {
				rl.close();
				resolve();
			});
		});

		await cleanupActor();
	} catch (error) {
		console.error("Error:", error);
		process.exit(1);
	}
}

run();
