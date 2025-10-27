#!/usr/bin/env tsx

import * as readline from "readline/promises";

const rl = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

const endpoint =
	process.env.RIVET_ENDPOINT ||
	(await rl.question("Endpoint (default: api.rivet.dev): ")).trim() ||
	"api.rivet.dev";

const rivetToken =
	process.env.RIVET_TOKEN ||
	(await rl.question("Rivet Token: ")).trim();

if (!rivetToken) {
	console.error("Error: Rivet Token is required");
	process.exit(1);
}

const actorId = (await rl.question("Actor ID: ")).trim();

if (!actorId) {
	console.error("Error: Actor ID is required");
	process.exit(1);
}

rl.close();

const url = endpoint.startsWith("http") ? endpoint : `https://${endpoint}`;

// Print equivalent curl command
console.log("Equivalent curl command:");
console.log(
	`curl -H "x-rivet-target: actor" -H "x-rivet-actor: ${actorId}" -H "x-rivet-token: ${rivetToken}" ${url}`,
);
console.log();

const response = await fetch(url, {
	method: "GET",
	headers: {
		"x-rivet-target": "actor",
		"x-rivet-actor": actorId,
		"x-rivet-token": rivetToken,
	},
});

if (!response.ok) {
	console.error(`Error: ${response.status} ${response.statusText}`);
	console.error(await response.text());
	process.exit(1);
}

const data = await response.text();

console.log(data);
