import fs from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import puppeteer from "puppeteer";

const SCREENSHOT_WIDTH = 1200;
const SCREENSHOT_HEIGHT = 675; // 16:9 aspect ratio
const DEV_SERVER_TIMEOUT = 60000; // 60 seconds to start dev server
const SCREENSHOT_DELAY = 3000; // Wait 3 seconds after page load for rendering

async function waitForServer(url: string, timeout: number): Promise<boolean> {
	const startTime = Date.now();

	while (Date.now() - startTime < timeout) {
		try {
			const response = await fetch(url);
			if (response.ok || response.status === 404) {
				return true;
			}
		} catch {
			// Server not ready yet
		}
		await new Promise((resolve) => setTimeout(resolve, 500));
	}

	return false;
}

async function takeScreenshot(
	exampleName: string,
	examplePath: string,
	outputPath: string,
): Promise<void> {
	console.log(`📸 Taking screenshot for ${exampleName}...`);

	// Check if package.json has a dev script
	const packageJsonPath = path.join(examplePath, "package.json");
	const packageJson = JSON.parse(
		await fs.readFile(packageJsonPath, "utf-8"),
	);

	if (!packageJson.scripts?.dev) {
		console.log(`⚠️  Skipping ${exampleName}: no dev script found`);
		return;
	}

	// Start dev server
	console.log(`🚀 Starting dev server for ${exampleName}...`);
	const devProcess = spawn("pnpm", ["dev"], {
		cwd: examplePath,
		stdio: ["ignore", "pipe", "pipe"],
		shell: true,
	});

	let serverUrl = "http://localhost:3000";
	let serverReady = false;

	// Capture output to detect server URL and readiness
	const outputHandler = (data: Buffer) => {
		const output = data.toString();
		console.log(`[${exampleName}] ${output.trim()}`);

		// Try to detect common dev server URLs
		const urlMatch = output.match(/https?:\/\/localhost:\d+/);
		if (urlMatch) {
			serverUrl = urlMatch[0];
		}
	};

	devProcess.stdout?.on("data", outputHandler);
	devProcess.stderr?.on("data", outputHandler);

	try {
		// Wait for server to be ready
		console.log(`⏳ Waiting for server at ${serverUrl}...`);
		serverReady = await waitForServer(serverUrl, DEV_SERVER_TIMEOUT);

		if (!serverReady) {
			throw new Error(
				`Dev server did not start within ${DEV_SERVER_TIMEOUT}ms`,
			);
		}

		console.log(`✅ Server ready at ${serverUrl}`);

		// Additional delay for app to fully render
		await new Promise((resolve) => setTimeout(resolve, SCREENSHOT_DELAY));

		// Launch browser and take screenshot
		const browser = await puppeteer.launch({
			headless: true,
			args: ["--no-sandbox", "--disable-setuid-sandbox"],
		});

		const page = await browser.newPage();
		await page.setViewport({
			width: SCREENSHOT_WIDTH,
			height: SCREENSHOT_HEIGHT,
		});

		await page.goto(serverUrl, { waitUntil: "networkidle0" });

		// Additional delay for any animations or async content
		await new Promise((resolve) => setTimeout(resolve, 2000));

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

export async function generateScreenshots(
	examplesDir: string,
	websitePublicDir: string,
) {
	console.log("\n🖼️  Generating screenshots...");

	// Ensure output directory exists
	await fs.mkdir(websitePublicDir, { recursive: true });

	// Read all example directories
	const entries = await fs.readdir(examplesDir, { withFileTypes: true });
	const exampleDirs = entries.filter((entry) => entry.isDirectory());

	for (const dir of exampleDirs) {
		const examplePath = path.join(examplesDir, dir.name);
		const packageJsonPath = path.join(examplePath, "package.json");

		// Skip if no package.json
		try {
			await fs.access(packageJsonPath);
		} catch {
			console.log(`⏭️  Skipping ${dir.name}: no package.json`);
			continue;
		}

		const outputDir = path.join(websitePublicDir, dir.name);
		const outputPath = path.join(outputDir, "image.png");

		// Check if screenshot already exists
		try {
			await fs.access(outputPath);
			console.log(`⏭️  Skipping ${dir.name}: screenshot already exists`);
			continue;
		} catch {
			// Screenshot doesn't exist, create it
		}

		// Create output directory
		await fs.mkdir(outputDir, { recursive: true });

		try {
			await takeScreenshot(dir.name, examplePath, outputPath);
		} catch (error) {
			console.error(`❌ Failed to generate screenshot for ${dir.name}:`, error);
			// Continue with next example
		}
	}

	console.log("\n✅ Screenshot generation complete!");
}
