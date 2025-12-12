#!/usr/bin/env tsx
/**
 * Script to take a screenshot of an example inside a Docker container.
 * This runs inside the container with isolated network space.
 *
 * Usage: tsx take-screenshot.ts <example-name> <output-path> <frontend-port> [timeout-ms]
 */

import fs from "node:fs/promises";
import path from "node:path";
import { spawn, type ChildProcess } from "node:child_process";
import puppeteer from "puppeteer";

const SCREENSHOT_WIDTH = 1200;
const SCREENSHOT_HEIGHT = 675; // 16:9 aspect ratio
const DEFAULT_DEV_SERVER_TIMEOUT = 60000; // 60 seconds to start dev server
const SCREENSHOT_DELAY = 3000; // Wait 3 seconds after page load for rendering

async function waitForServerWithPuppeteer(
	url: string,
	timeout: number,
	browser: puppeteer.Browser
): Promise<boolean> {
	const startTime = Date.now();
	let lastError: string = "";
	let attempts = 0;
	const page = await browser.newPage();

	try {
		while (Date.now() - startTime < timeout) {
			attempts++;
			try {
				const response = await page.goto(url, {
					waitUntil: "domcontentloaded",
					timeout: 5000
				});
				if (response && (response.ok() || response.status() === 404)) {
					console.log(`✅ Server responded after ${attempts} attempts (status: ${response.status()})`);
					return true;
				}
				lastError = `status ${response?.status() ?? 'unknown'}`;
			} catch (error) {
				lastError = error instanceof Error ? error.message : String(error);
			}
			// Log every 5 attempts
			if (attempts % 5 === 0) {
				console.log(`⏳ Still waiting for server... (attempt ${attempts}, last error: ${lastError})`);
			}
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}

		console.log(`❌ Server never responded after ${attempts} attempts (last error: ${lastError})`);
		return false;
	} finally {
		await page.close();
	}
}

async function takeScreenshot(
	exampleName: string,
	examplePath: string,
	outputPath: string,
	frontendPort: number,
	timeout: number = DEFAULT_DEV_SERVER_TIMEOUT,
): Promise<void> {
	console.log(`📸 Taking screenshot for ${exampleName}...`);

	// Check if package.json has a dev script
	const packageJsonPath = path.join(examplePath, "package.json");
	const packageJson = JSON.parse(
		await fs.readFile(packageJsonPath, "utf-8"),
	);

	if (!packageJson.scripts?.dev) {
		throw new Error(`No dev script found for ${exampleName}`);
	}

	// Build server URL from the specified port
	const serverUrl = `http://127.0.0.1:${frontendPort}`;
	console.log(`🔗 Using server URL: ${serverUrl}`);

	// Start dev server
	console.log(`🚀 Starting dev server for ${exampleName}...`);
	const devProcess = spawn("pnpm", ["dev"], {
		cwd: examplePath,
		stdio: ["ignore", "pipe", "pipe"],
		shell: true,
	});

	let processExited = false;
	let processError: Error | null = null;

	// Track if process exits prematurely
	devProcess.on("exit", (code) => {
		processExited = true;
		if (code !== 0 && code !== null) {
			processError = new Error(`Dev server exited with code ${code}`);
		}
	});

	// Log output for debugging
	const outputHandler = (data: Buffer) => {
		const output = data.toString();
		console.log(`[${exampleName}] ${output.trim()}`);
	};

	devProcess.stdout?.on("data", outputHandler);
	devProcess.stderr?.on("data", outputHandler);

	try {
		// Give dev server a moment to start
		console.log(`⏳ Waiting for dev server to start (timeout: ${timeout}ms)...`);
		await new Promise((resolve) => setTimeout(resolve, 2000));

		if (processError) {
			throw processError;
		}

		if (processExited) {
			throw new Error(`Dev server exited unexpectedly before starting`);
		}

		// Launch browser early for health checking and screenshots
		console.log(`🌐 Launching browser...`);
		const browser = await puppeteer.launch({
			headless: true,
			executablePath: process.env.PUPPETEER_EXECUTABLE_PATH || "/usr/bin/chromium",
			args: [
				"--no-sandbox",
				"--disable-setuid-sandbox",
				"--disable-dev-shm-usage",
				"--disable-gpu",
			],
		});

		// Wait for server to be ready using Puppeteer
		console.log(`⏳ Waiting for server at ${serverUrl}...`);
		const serverReady = await waitForServerWithPuppeteer(serverUrl, timeout, browser);

		if (!serverReady) {
			await browser.close();
			throw new Error(
				`Dev server did not respond on port ${frontendPort} within ${timeout}ms`,
			);
		}

		console.log(`✅ Server ready at ${serverUrl}`);

		// Additional delay for app to fully render
		await new Promise((resolve) => setTimeout(resolve, SCREENSHOT_DELAY));

		const page = await browser.newPage();
		await page.setViewport({
			width: SCREENSHOT_WIDTH,
			height: SCREENSHOT_HEIGHT,
			deviceScaleFactor: 2, // Capture at 2x resolution
		});

		// Don't use networkidle0 since some have chatty network interfaces
		const response = await page.goto(serverUrl, { waitUntil: "load" });

		// Accept 200-level responses and 304 (Not Modified, which is fine for cached content)
		const status = response?.status() ?? 0;
		if (!response || (status < 200 || (status >= 300 && status !== 304))) {
			throw new Error(
				`Frontend returned non-success status: ${status} for ${exampleName}`,
			);
		}

		// Additional delay for any animations or async content
		await new Promise((resolve) => setTimeout(resolve, 2000));

		// Ensure output directory exists
		await fs.mkdir(path.dirname(outputPath), { recursive: true });

		await page.screenshot({
			path: outputPath,
			type: "png",
		});

		await browser.close();

		console.log(`✅ Screenshot saved to ${outputPath}`);
	} finally {
		// Kill dev server
		devProcess.kill("SIGTERM");

		// Give it a moment to clean up
		await new Promise((resolve) => setTimeout(resolve, 1000));

		// Force kill if still running
		if (!devProcess.killed) {
			devProcess.kill("SIGKILL");
		}
	}
}

async function main() {
	const args = process.argv.slice(2);

	if (args.length < 3) {
		console.error("Usage: tsx take-screenshot.ts <example-name> <output-path> <frontend-port> [timeout-ms]");
		process.exit(1);
	}

	const [exampleName, outputPath, portArg, timeoutArg] = args;
	const frontendPort = parseInt(portArg, 10);
	const timeout = timeoutArg ? parseInt(timeoutArg, 10) : DEFAULT_DEV_SERVER_TIMEOUT;
	const examplePath = path.join("/app/examples", exampleName);

	if (isNaN(frontendPort) || frontendPort < 1 || frontendPort > 65535) {
		console.error(`Invalid frontend port: ${portArg}`);
		process.exit(1);
	}

	console.log(`📋 Config: example=${exampleName}, port=${frontendPort}, timeout=${timeout}ms`);

	// Check if example exists
	try {
		await fs.access(examplePath);
	} catch {
		console.error(`Example not found: ${examplePath}`);
		process.exit(1);
	}

	try {
		await takeScreenshot(exampleName, examplePath, outputPath, frontendPort, timeout);
		console.log("✅ Screenshot complete!");
		process.exit(0);
	} catch (error) {
		console.error(`❌ Failed to take screenshot:`, error);
		process.exit(1);
	}
}

main();
