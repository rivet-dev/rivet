#!/usr/bin/env tsx

import * as readline from "readline/promises";

const rl = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

const rivetToken = process.env.RIVET_TOKEN;
if (!rivetToken) {
	console.error("Error: RIVET_TOKEN environment variable is not set");
	process.exit(1);
}

const endpoint =
	process.env.RIVET_ENDPOINT ||
	(await rl.question("Rivet Endpoint (default: https://api.rivet.gg): ")) ||
	"https://api.rivet.gg";
const namespace =
	(await rl.question("Namespace (default: default): ")) || "default";
const runnerName = await rl.question("Runner name to delete: ");

rl.close();

if (!runnerName) {
	console.error("Error: Runner name is required");
	process.exit(1);
}

const response = await fetch(
	`${endpoint}/runner-configs/${runnerName}?namespace=${namespace}`,
	{
		method: "DELETE",
		headers: {
			Authorization: `Bearer ${rivetToken}`,
		},
	},
);

if (!response.ok) {
	console.error(`Error: ${response.status} ${response.statusText}`);
	console.error(await response.text());
	process.exit(1);
}

console.log(`✅ Successfully deleted runner configuration "${runnerName}"!`);