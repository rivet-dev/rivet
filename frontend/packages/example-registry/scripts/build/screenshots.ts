import fs from "node:fs/promises";
import path from "node:path";
import { spawn, execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const DOCKER_IMAGE_NAME = "rivet-example-screenshot";
const DOCKER_CONTEXT_PATH = path.join(__dirname, "../../../../../"); // Repository root

interface DockerBuildResult {
	success: boolean;
	error?: string;
}

async function buildDockerImage(): Promise<DockerBuildResult> {
	console.log("🐳 Building Docker image for screenshots...");

	return new Promise((resolve) => {
		const dockerfilePath = path.join(__dirname, "../screenshotter/Dockerfile");

		const buildProcess = spawn(
			"docker",
			[
				"build",
				"-t",
				DOCKER_IMAGE_NAME,
				"-f",
				dockerfilePath,
				DOCKER_CONTEXT_PATH,
			],
			{
				stdio: ["ignore", "pipe", "pipe"],
			},
		);

		let stderr = "";

		buildProcess.stdout?.on("data", (data) => {
			console.log(data.toString().trim());
		});

		buildProcess.stderr?.on("data", (data) => {
			stderr += data.toString();
			console.error(data.toString().trim());
		});

		buildProcess.on("close", (code) => {
			if (code === 0) {
				console.log("✅ Docker image built successfully");
				resolve({ success: true });
			} else {
				resolve({
					success: false,
					error: `Docker build failed with code ${code}: ${stderr}`,
				});
			}
		});

		buildProcess.on("error", (error) => {
			resolve({
				success: false,
				error: `Failed to start docker build: ${error.message}`,
			});
		});
	});
}

async function takeScreenshotInDocker(
	exampleName: string,
	outputPath: string,
	frontendPort: number,
	timeout?: number,
): Promise<{ success: boolean; error?: string }> {
	console.log(`📸 Taking screenshot for ${exampleName} in Docker...`);

	return new Promise((resolve) => {
		// Create a unique container name to avoid conflicts
		const containerName = `screenshot-${exampleName}-${Date.now()}`;

		// Output path inside container
		const containerOutputPath = `/output/image.png`;

		// Create a temporary directory for output
		const tempDir = `/tmp/rivet-screenshots-${Date.now()}`;

		try {
			// Create temp directory
			execSync(`mkdir -p ${tempDir}`);

			const runProcess = spawn(
				"docker",
				[
					"run",
					"--rm",
					"--name",
					containerName,
					// Mount output directory
					"-v",
					`${tempDir}:/output`,
					// Each container runs in isolated bridge network by default
					// This provides network isolation between concurrent screenshot processes
					// Security options for Chromium
					"--security-opt",
					"seccomp=unconfined",
					// Set memory limit to prevent runaway processes
					"--memory",
					"4g",
					// Set timeout via entrypoint timeout
					"--stop-timeout",
					"120",
					DOCKER_IMAGE_NAME,
					"npx",
					"tsx",
					"/app/frontend/packages/example-registry/scripts/screenshotter/take-screenshot.ts",
					exampleName,
					containerOutputPath,
					frontendPort.toString(),
					...(timeout ? [timeout.toString()] : []),
				],
				{
					stdio: ["ignore", "pipe", "pipe"],
				},
			);

			let stdout = "";
			let stderr = "";

			runProcess.stdout?.on("data", (data) => {
				stdout += data.toString();
				console.log(`[${exampleName}] ${data.toString().trim()}`);
			});

			runProcess.stderr?.on("data", (data) => {
				stderr += data.toString();
				console.error(`[${exampleName}] ${data.toString().trim()}`);
			});

			runProcess.on("close", async (code) => {
				if (code === 0) {
					// Copy screenshot from temp directory to final output
					try {
						const tempScreenshot = path.join(tempDir, "image.png");
						await fs.mkdir(path.dirname(outputPath), { recursive: true });
						await fs.copyFile(tempScreenshot, outputPath);
						await fs.rm(tempDir, { recursive: true, force: true });
						console.log(`✅ Screenshot saved to ${outputPath}`);
						resolve({ success: true });
					} catch (copyError) {
						resolve({
							success: false,
							error: `Failed to copy screenshot: ${copyError}`,
						});
					}
				} else {
					// Cleanup temp directory
					try {
						await fs.rm(tempDir, { recursive: true, force: true });
					} catch {
						// Ignore cleanup errors
					}
					resolve({
						success: false,
						error: `Docker run failed with code ${code}: ${stderr || stdout}`,
					});
				}
			});

			runProcess.on("error", (error) => {
				resolve({
					success: false,
					error: `Failed to start docker run: ${error.message}`,
				});
			});
		} catch (error) {
			resolve({
				success: false,
				error: `Setup failed: ${error}`,
			});
		}
	});
}

export interface ScreenshotOptions {
	/** Only generate screenshot for this specific example */
	singleExample?: string;
	/** Timeout in milliseconds for dev server to start */
	timeout?: number;
	/** Skip building the Docker image (useful if already built) */
	skipBuild?: boolean;
}

export async function generateScreenshots(
	examplesDir: string,
	websitePublicDir: string,
	options: ScreenshotOptions = {},
) {
	const { singleExample, timeout, skipBuild } = options;

	if (singleExample) {
		console.log(`\n🖼️  Generating screenshot for single example: ${singleExample}${timeout ? ` (timeout: ${timeout}ms)` : ''}...`);
	} else {
		console.log("\n🖼️  Generating screenshots using Docker...");
	}

	// First, build the Docker image (unless skipped)
	if (!skipBuild) {
		const buildResult = await buildDockerImage();
		if (!buildResult.success) {
			console.error(`❌ Failed to build Docker image: ${buildResult.error}`);
			console.log("⚠️  Skipping screenshot generation");
			return;
		}
	} else {
		console.log("⏭️  Skipping Docker image build");
	}

	// Ensure output directory exists
	await fs.mkdir(websitePublicDir, { recursive: true });

	// Read all example directories (or just the single one if specified)
	const entries = await fs.readdir(examplesDir, { withFileTypes: true });
	let exampleDirs = entries.filter((entry) => entry.isDirectory());

	// Filter to single example if specified
	if (singleExample) {
		exampleDirs = exampleDirs.filter((entry) => entry.name === singleExample);
		if (exampleDirs.length === 0) {
			console.error(`❌ Example not found: ${singleExample}`);
			return;
		}
	}

	const results: Array<{
		example: string;
		success: boolean;
		error?: string;
	}> = [];

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

		// Skip if noFrontend is set
		const packageJson = JSON.parse(
			await fs.readFile(packageJsonPath, "utf-8"),
		);
		if (packageJson.template?.noFrontend) {
			console.log(`⏭️  Skipping ${dir.name}: noFrontend is set`);
			continue;
		}

		// Skip if no dev script
		if (!packageJson.scripts?.dev) {
			console.log(`⏭️  Skipping ${dir.name}: no dev script`);
			continue;
		}

		// Get frontend port from template config
		const frontendPort = packageJson.template?.frontendPort;
		if (!frontendPort) {
			console.log(`⏭️  Skipping ${dir.name}: no frontendPort specified`);
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

		// Take screenshot in Docker
		const result = await takeScreenshotInDocker(dir.name, outputPath, frontendPort, timeout);
		results.push({
			example: dir.name,
			...result,
		});

		if (!result.success) {
			console.error(`❌ Failed to generate screenshot for ${dir.name}: ${result.error}`);
		}
	}

	// Summary
	const successful = results.filter((r) => r.success).length;
	const failed = results.filter((r) => !r.success).length;

	console.log(`\n📊 Screenshot generation summary:`);
	console.log(`   ✅ Successful: ${successful}`);
	console.log(`   ❌ Failed: ${failed}`);

	if (failed > 0) {
		console.log(`\n❌ Failed examples:`);
		for (const result of results.filter((r) => !r.success)) {
			console.log(`   - ${result.example}: ${result.error}`);
		}
	}

	console.log("\n✅ Screenshot generation complete!");
}
