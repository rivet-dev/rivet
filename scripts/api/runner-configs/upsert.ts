#!/usr/bin/env tsx

import * as readline from "readline/promises";

const rl = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

async function ask(
	question: string,
	options: { default?: string; allowEmpty?: boolean } = {},
) {
	const suffix =
		options.default !== undefined && options.default !== ""
			? ` (default: ${options.default})`
			: "";
	const answer = (await rl.question(`${question}${suffix}: `)).trim();
	if (answer === "" && options.default !== undefined) {
		return options.default;
	}
	if (answer === "" && options.allowEmpty) {
		return "";
	}
	return answer;
}

async function askNumber(question: string, defaultValue: number) {
	const answer = (
		await rl.question(`${question} (default: ${defaultValue}): `)
	).trim();
	if (answer === "") {
		return defaultValue;
	}
	const value = Number(answer);
	if (!Number.isFinite(value) || Number.isNaN(value)) {
		console.error(`Error: ${question} must be a number`);
		rl.close();
		process.exit(1);
	}
	return value;
}

function parseJson(input: string, context: string) {
	try {
		return JSON.parse(input);
	} catch (error) {
		console.error(`Error: unable to parse ${context} JSON`);
		rl.close();
		process.exit(1);
	}
}

function ensureStringRecord(
	value: unknown,
	context: string,
): Record<string, string> {
	if (value === null || typeof value !== "object" || Array.isArray(value)) {
		console.error(
			`Error: ${context} must be a JSON object with string values`,
		);
		rl.close();
		process.exit(1);
	}
	const record: Record<string, string> = {};
	for (const [key, val] of Object.entries(value)) {
		if (typeof val !== "string") {
			console.error(
				`Error: ${context} value for key "${key}" must be a string`,
			);
			rl.close();
			process.exit(1);
		}
		record[key] = val;
	}
	return record;
}

const rivetToken =
	process.env.RIVET_TOKEN || (await ask("Rivet token", { default: "dev" }));

const endpoint =
	process.env.RIVET_ENDPOINT ||
	(await ask("Rivet endpoint", { default: "http://localhost:6420" }));
const namespace = await ask("Namespace", { default: "default" });
const datacenter = await ask("Datacenter", { default: "default" });
const runnerName = await ask("Runner name", { default: "serverless" });
const runnerType = (
	await ask("Runner config type (normal/serverless)", {
		default: "serverless",
	})
).toLowerCase();

if (runnerType !== "normal" && runnerType !== "serverless") {
	console.error(
		"Error: runner config type must be either 'normal' or 'serverless'",
	);
	rl.close();
	process.exit(1);
}

const metadataInput = await ask("Metadata JSON (optional)", {
	allowEmpty: true,
});
let metadata: unknown;
if (metadataInput) {
	metadata = parseJson(metadataInput, "metadata");
}

let dcRunnerConfig: Record<string, unknown>;

if (runnerType === "normal") {
	dcRunnerConfig = {
		normal: {},
		...(metadata !== undefined ? { metadata } : {}),
	};
} else {
	const serverlessUrl = await ask("Serverless URL", {
		default: "http://localhost:3000/api/rivet/start",
	});
	const headersInput = await ask("Serverless headers JSON", {
		default: "{}",
	});
	const headers = ensureStringRecord(
		parseJson(headersInput, "headers"),
		"headers",
	);
	const requestLifespan = await askNumber(
		"Request lifespan (seconds)",
		15 * 60,
	);
	const slotsPerRunner = await askNumber("Slots per runner", 100);
	const minRunners = await askNumber("Min runners", 1);
	const maxRunners = await askNumber("Max runners", 3);
	const runnersMargin = await askNumber("Runners margin", 1);

	dcRunnerConfig = {
		serverless: {
			url: serverlessUrl,
			headers,
			request_lifespan: requestLifespan,
			slots_per_runner: slotsPerRunner,
			min_runners: minRunners,
			max_runners: maxRunners,
			runners_margin: runnersMargin,
		},
		...(metadata !== undefined ? { metadata } : {}),
	};
}

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
			datacenters: {
				[datacenter]: dcRunnerConfig,
			},
		}),
	},
);

if (!response.ok) {
	console.error(`Error: ${response.status} ${response.statusText}`);
	console.error(await response.text());
	process.exit(1);
}

console.log("✅ Successfully upserted runner config!");
