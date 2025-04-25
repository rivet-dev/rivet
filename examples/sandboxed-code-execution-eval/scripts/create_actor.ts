#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read --allow-run

// Import necessary modules
import { RivetClient } from "npm:@rivet-gg/api@24.6.2"

async function main() {
	const endpoint = Deno.env.get("RIVET_ENDPOINT");
	const token = Deno.env.get("RIVET_SERVICE_TOKEN");
	const project = Deno.env.get("RIVET_PROJECT");
	const environment = Deno.env.get("RIVET_ENVIRONMENT");

	if (!endpoint || !token || !project || !environment) throw new Error("Missing required env var");

	const client = new RivetClient({
		environment: endpoint,
		token,
	});

	// Find the nearest region (sorted by best to worst)
	console.log("Resolving region");
	const { regions: [region] } = await client.actor.regions.list({ project, environment });
	
	// Create actor
	console.log("Creating");
	const { actor } = await client.actor.create({
		project,
		environment,
		body: {
			// This may be whatever you like
			tags: { name: "app", foo: "bar" },
			// Must match the name in rivet.json
			buildTags: { name: "app" },
			region: region.id,
			runtime: {
				environment: {
					USER_CODE_FILE_NAME: "date.ts",
				},
			},
			network: {
				ports: {
					http: { protocol: "https" }
				}
			},
			resources: {
				cpu: 1000,
				memory: 2048,
			}
		}
	});

	// Ping actor
	console.log("Pinging");
	const response = await fetch(
		actor.network.ports.http.url!,
		{
			method: "POST",
			body: "foo",
		},
	);
	const responseBody = await response.text();
	console.log("Response", responseBody);

	// Wait
	console.log("Sleeping for 5 seconds before destroying.");
	await new Promise((resolve) => setTimeout(resolve, 5000));

	// Destroy actor
	console.log("Destroying");
	await client.actor.destroy(actor.id, { project, environment })
}

await main();
