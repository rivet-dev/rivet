#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read --allow-run

// Import necessary modules
import { resolve } from "https://deno.land/std@0.114.0/path/mod.ts";
import { v4 as uuidv4 } from "https://deno.land/std@0.114.0/uuid/mod.ts";

// Constants
const ENDPOINT = Deno.env.get("RIVET_ENDPOINT") ?? "https://api.rivet.gg";
const BUILD =
	Deno.env.get("RIVET_BUILD") ??
	resolve(import.meta.dirname, "./fixtures/echo_http.js");

//const PROJECT = "dreamlab-dem-qmv";
const PROJECT = "dreamlab-dem-qmv";
const ENV = "prod";
const TOKEN = process.env.RIVET_SERVICE_TOKEN;

// Helper function to make HTTP requests
async function httpRequest(method: string, url: string, body?: any) {
	const fullUrl = new URL(url);
	fullUrl.searchParams.set("project", PROJECT);
	fullUrl.searchParams.set("environment", ENV);

	console.log(
		`Request: ${method} ${fullUrl.toString()}\n${JSON.stringify(body)}`,
	);

	const response = await fetch(fullUrl.toString(), {
		method,
		headers: {
			"Content-Type": "application/json",
			Authorization: `Bearer ${TOKEN}`,
		},
		body: body ? JSON.stringify(body) : undefined,
	});
	const responseText = await response.text();

	console.log(`Response: ${response.status}\n${responseText}`);

	if (!response.ok) {
		throw new Error(`HTTP status: ${response.status}\n\nBody: ${responseText}`);
	}

	console.log();

	return JSON.parse(responseText);
}

async function listRegions() {
	const response = await httpRequest("GET", `${ENDPOINT}/regions`);
	return response.regions;
}

async function uploadBuild() {
	// Copy file to tmp directory and rename to index.js
	const tmpDir = await Deno.makeTempDir();
	const tmpFilePath = resolve(tmpDir, "index.js");
	await Deno.copyFile(BUILD, tmpFilePath);

	// Archive code
	const bundleLocation = resolve(tmpDir, "bundle.tar");
	const tarCommand = new Deno.Command("tar", {
		args: ["cf", bundleLocation, "-C", tmpDir, "index.js"],
	});
	const { code } = await tarCommand.output();
	console.assert(code === 0);

	const buildContent = await Deno.readFile(bundleLocation);
	const contentLength = buildContent.length;

	const randomString = crypto.randomUUID().replace(/-/g, "").slice(0, 8);
	const { build, presigned_requests } = await httpRequest(
		"POST",
		`${ENDPOINT}/builds/prepare`,
		{
			image_file: {
				content_length: contentLength,
				path: "bundle.tar",
			},
			kind: "javascript",
			name: `build-${randomString}`,
		},
	);

	await fetch(presigned_requests[0].url, {
		method: "PUT",
		body: buildContent,
	});

	await httpRequest("POST", `${ENDPOINT}/builds/${build}/complete`, {});

	return { buildId: build };
}

async function createActor(region: string, buildId: string) {
	const createResponse = await httpRequest("POST", `${ENDPOINT}/actors`, {
		tags: {},
		region,
		network: {
			mode: "bridge",
			ports: {
				http: { protocol: "https" },
			},
		},
		build: buildId,
	});

	while (true) {
		const { actor } = await httpRequest(
			"GET",
			`${ENDPOINT}/actors/${createResponse.actor.id}`,
		);
		if (actor.network.ports.http.hostname != null) {
			return actor;
		} else {
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
	}
}

async function pingActor(actor) {
	while (true) {
		try {
			console.log("Pinging actor");

			const response = await fetch(
				actor.network.ports.http.url,
				{
					method: "POST",
					body: "foo",
				},
			);
			const responseBody = await response.text();
			// Validate the response
			if (responseBody === "foo") {
				console.log("Response validated successfully.");
			} else {
				console.error("Response validation failed.");
			}

			console.log();

			break;
		} catch (err) {
			console.log("Failed to ping actor:", err);
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
	}
}

async function destroyActor(actor) {
	await httpRequest("DELETE", `${ENDPOINT}/actors/${actor.id}`);
}

async function main() {
	const { buildId } = await uploadBuild();

	const regions = await listRegions();
	const actor = await createActor(regions[0].id, buildId);

	await pingActor(actor);

	console.log("Sleeping for 5 seconds before destroying.");
	await new Promise((resolve) => setTimeout(resolve, 5000));

	await destroyActor(actor);
}

await main();
