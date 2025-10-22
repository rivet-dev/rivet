#!/usr/bin/env tsx

import * as readline from "readline/promises";

const rl = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

const rivetToken =
	process.env.RIVET_TOKEN ||
	(await rl.question("Rivet Token (default: dev): ")).trim() ||
	"dev";

const endpoint =
	process.env.RIVET_ENDPOINT ||
	(await rl.question("Rivet Endpoint (default: http://localhost:6420): ")) ||
	"http://localhost:6420";
const namespace =
	(await rl.question("Namespace (default: default): ")) || "default";

rl.close();

const response = await fetch(`${endpoint}/runners?namespace=${namespace}`, {
	method: "GET",
	headers: {
		Authorization: `Bearer ${rivetToken}`,
	},
});

if (!response.ok) {
	console.error(`Error: ${response.status} ${response.statusText}`);
	console.error(await response.text());
	process.exit(1);
}

const data = await response.json();

// Just show the raw formatted JSON
console.log(JSON.stringify(data, null, 2));
