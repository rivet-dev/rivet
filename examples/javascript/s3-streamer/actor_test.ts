import { RivetClient } from "@rivet-gg/api";

// Get bucket and key from command line arguments
const bucket = Deno.args[0];
const key = Deno.args[1];

if (!bucket || !key) {
	throw new Error("Usage: actor_test.ts <bucket> <key>");
}

// Validate required environment variables
const RIVET_ENDPOINT = Deno.env.get("RIVET_ENDPOINT");
const RIVET_SERVICE_TOKEN = Deno.env.get("RIVET_SERVICE_TOKEN");
const RIVET_PROJECT = Deno.env.get("RIVET_PROJECT");
const RIVET_ENVIRONMENT = Deno.env.get("RIVET_ENVIRONMENT");
const AWS_ACCESS_KEY_ID = Deno.env.get("AWS_ACCESS_KEY_ID");
const AWS_SECRET_ACCESS_KEY = Deno.env.get("AWS_SECRET_ACCESS_KEY");
const AWS_REGION = Deno.env.get("AWS_REGION") || "us-east-1";
const AWS_ENDPOINT = Deno.env.get("AWS_ENDPOINT");

if (!RIVET_ENDPOINT) throw new Error("missing RIVET_ENDPOINT");
if (!RIVET_SERVICE_TOKEN) throw new Error("missing RIVET_SERVICE_TOKEN");
if (!RIVET_PROJECT) throw new Error("missing RIVET_PROJECT");
if (!RIVET_ENVIRONMENT) throw new Error("missing RIVET_ENVIRONMENT");
if (!AWS_ACCESS_KEY_ID) throw new Error("missing AWS_ACCESS_KEY_ID");
if (!AWS_SECRET_ACCESS_KEY) throw new Error("missing AWS_SECRET_ACCESS_KEY");

// Optional region override
const region = Deno.env.get("REGION") || undefined;

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
					name: "s3-streamer",
				},
				buildTags: { name: "s3-streamer", current: "true" },
				runtime: {
					environment: {
						AWS_ACCESS_KEY_ID,
						AWS_SECRET_ACCESS_KEY,
						AWS_REGION,
						AWS_ENDPOINT,
					},
				},
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
				lifecycle: {
					durable: false,
				},
			},
		});
		actorId = actor.id;

		const port = actor.network.ports.http;
		const actorOrigin = `${port.protocol}://${port.hostname}:${port.port}${port.path ?? ""}`;
		console.log("Created actor at", actorOrigin);

		// Wait for actor to be ready
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

		console.log("Testing S3 streaming...");
		console.log(`Streaming from S3: ${bucket}/${key}`);
		const response = await fetch(`${actorOrigin}/s3-test/${bucket}/${key}`);

		if (response.ok) {
			const text = await response.text();
			console.log("Content length:", text.length);
		} else {
			console.error("S3 streaming failed:", response.status);
			throw new Error(`S3 streaming failed: ${response.status}`);
		}
	} catch (error) {
		console.error("Error:", error);
		throw error;
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

// Run single test
await run();
