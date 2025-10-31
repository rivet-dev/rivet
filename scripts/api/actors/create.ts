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
	process.env.RIVET_NAMESPACE ||
	(await rl.question("Namespace (default: default): ")) || "default";

const name = (await rl.question("Actor Name: ")).trim();
if (!name) {
	console.error("Error: Actor Name is required");
	process.exit(1);
}

const runnerNameSelector =
	(await rl.question("Runner Name Selector (default: default): ")).trim() ||
	"default";

const crashPolicy =
	(await rl.question("Crash Policy (default: restart): ")).trim() || "restart";

const datacenter = (await rl.question("Datacenter (optional): ")).trim() || undefined;

const key = (await rl.question("Key (optional): ")).trim() || undefined;

const input = (await rl.question("Input JSON (optional): ")).trim() || undefined;

rl.close();

const body = {
	name,
	runner_name_selector: runnerNameSelector,
	crash_policy: crashPolicy,
	...(datacenter && { datacenter }),
	...(key && { key }),
	...(input && { input }),
};

const response = await fetch(
	`${endpoint}/actors?namespace=${namespace}`,
	{
		method: "POST",
		headers: {
			Authorization: `Bearer ${rivetToken}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify(body),
	},
);

if (!response.ok) {
	console.error(`Error: ${response.status} ${response.statusText}`);
	console.error(await response.text());
	process.exit(1);
}

const data = await response.json();

console.log(JSON.stringify(data, null, 2));
