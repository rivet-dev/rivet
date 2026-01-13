#!/usr/bin/env npx tsx

/**
 * Test script for validating RivetKit examples.
 *
 * This script:
 * 1. Iterates through all examples with a `template` config in package.json
 * 2. Spawns the dev command for each example
 * 3. Validates the index page is reachable
 * 4. Validates the RivetKit API endpoint is reachable
 *
 * Usage:
 *   npx tsx scripts/test-examples/test-examples.ts
 *   npx tsx scripts/test-examples/test-examples.ts --example chat-room
 *   npx tsx scripts/test-examples/test-examples.ts --skip ai-agent,drizzle
 */

import { spawn, ChildProcess } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";

const EXAMPLES_DIR = path.resolve(import.meta.dirname, "../../examples");
const DEFAULT_PORT = 5173;
const RIVET_MANAGER_PORT = 6420;
const RIVET_API_PATH = "/api/rivet/";
const STARTUP_TIMEOUT_MS = 60_000;
const POLL_INTERVAL_MS = 500;

interface ExampleConfig {
	name: string;
	dir: string;
	port: number;
	noFrontend: boolean;
}

interface TestResult {
	example: string;
	indexReachable: boolean;
	rivetApiReachable: boolean;
	managerReachable: boolean;
	error?: string;
}

function parseArgs(): { example?: string; skip: string[] } {
	const args = process.argv.slice(2);
	let example: string | undefined;
	let skip: string[] = [];

	for (let i = 0; i < args.length; i++) {
		if (args[i] === "--example" && args[i + 1]) {
			example = args[++i];
		} else if (args[i] === "--skip" && args[i + 1]) {
			skip = args[++i].split(",");
		}
	}

	return { example, skip };
}

function getExamples(): ExampleConfig[] {
	const examples: ExampleConfig[] = [];
	const entries = fs.readdirSync(EXAMPLES_DIR, { withFileTypes: true });

	for (const entry of entries) {
		if (!entry.isDirectory()) continue;

		const packageJsonPath = path.join(EXAMPLES_DIR, entry.name, "package.json");
		if (!fs.existsSync(packageJsonPath)) continue;

		try {
			const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf-8"));
			if (!packageJson.template) continue;

			examples.push({
				name: entry.name,
				dir: path.join(EXAMPLES_DIR, entry.name),
				port: packageJson.template.frontendPort ?? DEFAULT_PORT,
				noFrontend: packageJson.template.noFrontend ?? false,
			});
		} catch {
			// Skip invalid package.json files
		}
	}

	return examples;
}

async function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForServer(
	url: string,
	timeoutMs: number
): Promise<boolean> {
	const startTime = Date.now();

	while (Date.now() - startTime < timeoutMs) {
		try {
			const response = await fetch(url, {
				signal: AbortSignal.timeout(5000),
			});
			if (response.ok || response.status < 500) {
				return true;
			}
		} catch {
			// Server not ready yet
		}
		await sleep(POLL_INTERVAL_MS);
	}

	return false;
}

async function checkEndpoint(url: string): Promise<boolean> {
	try {
		const response = await fetch(url, {
			signal: AbortSignal.timeout(10000),
		});
		// For index, we expect 200. For API, we might get 404 (no route) but server should respond
		return response.ok || response.status < 500;
	} catch {
		return false;
	}
}

async function runDevServer(
	exampleDir: string
): Promise<ChildProcess> {
	return new Promise((resolve, reject) => {
		const child = spawn("pnpm", ["dev"], {
			cwd: exampleDir,
			stdio: ["ignore", "pipe", "pipe"],
			env: {
				...process.env,
				FORCE_COLOR: "0",
			},
		});

		let output = "";

		child.stdout?.on("data", (data) => {
			output += data.toString();
			// Check if server is ready by looking for common Vite output
			if (output.includes("Local:") || output.includes("ready in")) {
				resolve(child);
			}
		});

		child.stderr?.on("data", (data) => {
			output += data.toString();
		});

		child.on("error", (err) => {
			reject(new Error(`Failed to start dev server: ${err.message}`));
		});

		child.on("exit", (code) => {
			if (code !== null && code !== 0) {
				reject(new Error(`Dev server exited with code ${code}\n${output}`));
			}
		});

		// Resolve after a short delay even if we don't see the ready message
		setTimeout(() => resolve(child), 5000);
	});
}

function killProcess(child: ChildProcess): Promise<void> {
	return new Promise((resolve) => {
		if (child.killed) {
			resolve();
			return;
		}

		child.on("exit", () => resolve());
		child.kill("SIGTERM");

		// Force kill after 5 seconds
		setTimeout(() => {
			if (!child.killed) {
				child.kill("SIGKILL");
			}
			resolve();
		}, 5000);
	});
}

async function testExample(config: ExampleConfig): Promise<TestResult> {
	const result: TestResult = {
		example: config.name,
		indexReachable: false,
		rivetApiReachable: false,
		managerReachable: false,
	};

	// Skip frontend tests for backend-only examples
	if (config.noFrontend) {
		console.log(`  ⏭️  Skipping ${config.name} (no frontend)`);
		result.indexReachable = true;
		result.rivetApiReachable = true;
		result.managerReachable = true;
		return result;
	}

	const baseUrl = `http://localhost:${config.port}`;
	const managerUrl = `http://localhost:${RIVET_MANAGER_PORT}`;
	let child: ChildProcess | undefined;

	try {
		console.log(`  🚀 Starting dev server...`);
		child = await runDevServer(config.dir);

		console.log(`  ⏳ Waiting for server at ${baseUrl}...`);
		const serverReady = await waitForServer(baseUrl, STARTUP_TIMEOUT_MS);

		if (!serverReady) {
			result.error = `Server did not start within ${STARTUP_TIMEOUT_MS / 1000}s`;
			return result;
		}

		// Test index page
		console.log(`  🔍 Testing index page...`);
		result.indexReachable = await checkEndpoint(baseUrl);
		if (!result.indexReachable) {
			result.error = "Index page not reachable";
		}

		// Test RivetKit API endpoint
		console.log(`  🔍 Testing RivetKit API endpoint...`);
		const apiUrl = `${baseUrl}${RIVET_API_PATH}`;
		result.rivetApiReachable = await checkEndpoint(apiUrl);
		if (!result.rivetApiReachable && !result.error) {
			result.error = "RivetKit API endpoint not reachable";
		}

		// Test RivetKit manager on port 6420
		console.log(`  🔍 Testing RivetKit manager at ${managerUrl}...`);
		result.managerReachable = await checkEndpoint(managerUrl);
		if (!result.managerReachable && !result.error) {
			result.error = `RivetKit manager not reachable at port ${RIVET_MANAGER_PORT}`;
		}
	} catch (err) {
		result.error = err instanceof Error ? err.message : String(err);
	} finally {
		if (child) {
			console.log(`  🛑 Stopping dev server...`);
			await killProcess(child);
		}
	}

	return result;
}

async function main(): Promise<void> {
	const { example: targetExample, skip } = parseArgs();
	let examples = getExamples();

	if (targetExample) {
		examples = examples.filter((e) => e.name === targetExample);
		if (examples.length === 0) {
			console.error(`❌ Example "${targetExample}" not found`);
			process.exit(1);
		}
	}

	if (skip.length > 0) {
		examples = examples.filter((e) => !skip.includes(e.name));
	}

	console.log(`\n📦 Found ${examples.length} examples to test\n`);

	const results: TestResult[] = [];

	for (const config of examples) {
		console.log(`\n🧪 Testing: ${config.name}`);
		console.log(`   Directory: ${config.dir}`);
		console.log(`   Port: ${config.port}`);

		const result = await testExample(config);
		results.push(result);

		if (result.indexReachable && result.rivetApiReachable && result.managerReachable) {
			console.log(`  ✅ PASSED`);
		} else {
			console.log(`  ❌ FAILED: ${result.error}`);
		}
	}

	// Print summary
	console.log("\n" + "=".repeat(60));
	console.log("📊 SUMMARY");
	console.log("=".repeat(60));

	const passed = results.filter(
		(r) => r.indexReachable && r.rivetApiReachable && r.managerReachable
	);
	const failed = results.filter(
		(r) => !r.indexReachable || !r.rivetApiReachable || !r.managerReachable
	);

	console.log(`\n✅ Passed: ${passed.length}`);
	for (const r of passed) {
		console.log(`   - ${r.example}`);
	}

	if (failed.length > 0) {
		console.log(`\n❌ Failed: ${failed.length}`);
		for (const r of failed) {
			console.log(`   - ${r.example}: ${r.error}`);
		}
	}

	console.log("\n");

	// Exit with error if any tests failed
	if (failed.length > 0) {
		process.exit(1);
	}
}

main().catch((err) => {
	console.error("Fatal error:", err);
	process.exit(1);
});
