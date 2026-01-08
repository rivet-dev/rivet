#!/usr/bin/env -S npx tsx

/**
 * Generates a Vercel deploy button URL for a given example.
 *
 * Usage:
 *   npx tsx scripts/vercel/generate-deploy-url.ts <example-name>
 *
 * Example:
 *   npx tsx scripts/vercel/generate-deploy-url.ts chat-room
 */

const REPO_OWNER = "rivet-dev";
const REPO_NAME = "rivet";
const BRANCH = "01-07-chore_example_update_examples_to_use_srvx";

interface DeployUrlOptions {
	example: string;
	projectName?: string;
	env?: string[];
	envDescription?: string;
	envLink?: string;
	demoTitle?: string;
	demoDescription?: string;
	demoUrl?: string;
}

function generateDeployUrl(options: DeployUrlOptions): string {
	const { example, projectName, env, envDescription, envLink, demoTitle, demoDescription, demoUrl } = options;

	const baseUrl = "https://vercel.com/new/clone";

	// Build the repository URL with branch
	const repoUrl = `https://github.com/${REPO_OWNER}/${REPO_NAME}/tree/${BRANCH}/examples/${example}`;

	const params = new URLSearchParams();

	// Repository configuration
	params.set("repository-url", repoUrl);

	// Project name defaults to example name
	params.set("project-name", projectName ?? example);

	// Environment variables
	if (env && env.length > 0) {
		params.set("env", env.join(","));
	}
	if (envDescription) {
		params.set("envDescription", envDescription);
	}
	if (envLink) {
		params.set("envLink", envLink);
	}

	// Demo card
	if (demoTitle) {
		params.set("demo-title", demoTitle);
	}
	if (demoDescription) {
		params.set("demo-description", demoDescription);
	}
	if (demoUrl) {
		params.set("demo-url", demoUrl);
	}

	return `${baseUrl}?${params.toString()}`;
}

function generateMarkdownButton(url: string): string {
	return `[![Deploy with Vercel](https://vercel.com/button)](${url})`;
}

function generateHtmlButton(url: string): string {
	return `<a href="${url}"><img src="https://vercel.com/button" alt="Deploy with Vercel"/></a>`;
}

function main() {
	const args = process.argv.slice(2);

	if (args.length === 0) {
		console.error("Usage: npx tsx scripts/vercel/generate-deploy-url.ts <example-name>");
		console.error("");
		console.error("Example:");
		console.error("  npx tsx scripts/vercel/generate-deploy-url.ts chat-room");
		process.exit(1);
	}

	const example = args[0];

	const url = generateDeployUrl({
		example,
	});

	console.log("Deploy URL:");
	console.log(url);
	console.log("");
	console.log("Markdown:");
	console.log(generateMarkdownButton(url));
	console.log("");
	console.log("HTML:");
	console.log(generateHtmlButton(url));
}

main();
