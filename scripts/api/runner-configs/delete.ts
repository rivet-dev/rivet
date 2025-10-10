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
const runnerName = await rl.question("Runner name to delete: ");

if (!runnerName) {
	console.error("Error: Runner name is required");
	rl.close();
	process.exit(1);
}

const confirmDelete = await rl.question(
	`Are you sure you want to delete runner "${runnerName}" in namespace "${namespace}"? (yes/no): `,
);

rl.close();

if (confirmDelete.toLowerCase() !== "yes") {
	console.log("Deletion cancelled.");
	process.exit(0);
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
