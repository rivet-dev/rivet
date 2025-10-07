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
const runnerName =
	(await rl.question("Runner name (default: serverless): ")) || "serverless";
const serverlessUrl =
	(await rl.question(
		"Serverless URL (default: http://localhost:8080/api/start): ",
	)) || "http://localhost:8080/api/start";

rl.close();

const response = await fetch(
	`${endpoint}/runner-configs/${runnerName}?namespace=${namespace}`,
	{
		method: "PUT",
		headers: {
			Authorization: `Bearer ${rivetToken}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			serverless: {
				url: serverlessUrl,
				headers: {},
				runners_margin: 1,
				min_runners: 1,
				max_runners: 3,
				slots_per_runner: 100,
				request_lifespan: 15 * 60,
			},
		}),
	},
);

if (!response.ok) {
	console.error(`Error: ${response.status} ${response.statusText}`);
	console.error(await response.text());
	process.exit(1);
}

console.log("✅ Successfully configured serverless runner!");

