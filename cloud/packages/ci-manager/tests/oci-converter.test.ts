import { test, expect, beforeAll, afterAll } from "vitest";
import { execSync } from "child_process";
import { mkdir, writeFile, rm, readFile } from "fs/promises";
import { join } from "path";
import * as tar from "tar";
import { convertDockerTarToOCIBundle } from "../src/oci-converter";

const TEST_DIR = "/tmp/oci-converter-test";
const TEST_IMAGE_NAME = "oci-converter-test";

async function createTestDockerImage(): Promise<string> {
	const contextDir = join(TEST_DIR, "docker-context");
	await mkdir(contextDir, { recursive: true });
	
	// Create a simple Dockerfile
	const dockerfile = `
FROM alpine:latest
RUN echo "Hello from OCI converter test!" > /hello.txt
COPY test-script.sh /test-script.sh
RUN chmod +x /test-script.sh
CMD ["/test-script.sh"]
`;
	
	// Create a test script
	const testScript = `#!/bin/sh
echo "OCI conversion test successful!"
cat /hello.txt
`;
	
	await writeFile(join(contextDir, "Dockerfile"), dockerfile.trim());
	await writeFile(join(contextDir, "test-script.sh"), testScript.trim());
	
	// Build the Docker image
	console.log(`Building test Docker image: ${TEST_IMAGE_NAME}`);
	execSync(`docker build -t ${TEST_IMAGE_NAME} .`, {
		cwd: contextDir,
		stdio: "pipe"
	});
	
	// Save the Docker image to tar
	const dockerTarPath = join(TEST_DIR, "test-image.tar");
	console.log(`Saving Docker image to: ${dockerTarPath}`);
	execSync(`docker save -o ${dockerTarPath} ${TEST_IMAGE_NAME}`, {
		stdio: "pipe"
	});
	
	return dockerTarPath;
}

async function createMockKanikoOutput(dockerTarPath: string): Promise<string> {
	const kanikoOutputPath = join(TEST_DIR, "kaniko-output.tar.gz");
	const tempDir = join(TEST_DIR, "kaniko-temp");
	
	await mkdir(tempDir, { recursive: true });
	
	// Copy docker tar to the expected location inside kaniko output
	const dockerTarData = await readFile(dockerTarPath);
	await writeFile(join(tempDir, "image.tar"), dockerTarData);
	
	// Create kaniko output tar.gz
	await tar.create(
		{
			file: kanikoOutputPath,
			gzip: true,
			cwd: tempDir
		},
		["."]
	);
	
	// Cleanup temp directory
	await rm(tempDir, { recursive: true, force: true });
	
	return kanikoOutputPath;
}

async function validateOCIBundle(bundleTarPath: string): Promise<void> {
	const validateDir = join(TEST_DIR, "validate");
	await mkdir(validateDir, { recursive: true });
	
	try {
		// Extract the OCI bundle tar
		await tar.extract({
			file: bundleTarPath,
			cwd: validateDir
		});
		
		// Check for required OCI bundle files
		const configJsonPath = join(validateDir, "config.json");
		const rootfsPath = join(validateDir, "rootfs");
		
		// Verify config.json exists and is valid JSON
		const configData = await readFile(configJsonPath, "utf8");
		const config = JSON.parse(configData);
		
		expect(config).toBeDefined();
		expect(config.ociVersion).toBeDefined();
		expect(config.process).toBeDefined();
		expect(config.root).toBeDefined();
		
		// Verify rootfs directory exists
		const rootfsStat = await import("fs/promises").then(fs => fs.stat(rootfsPath));
		expect(rootfsStat.isDirectory()).toBe(true);
		
		console.log("OCI bundle validation passed");
		
	} finally {
		await rm(validateDir, { recursive: true, force: true });
	}
}

beforeAll(async () => {
	// Create test directory
	await mkdir(TEST_DIR, { recursive: true });
	
	// Check if Docker is available
	try {
		execSync("docker --version", { stdio: "pipe" });
	} catch (error) {
		throw new Error("Docker is not available. Please install Docker to run OCI converter tests.");
	}
	
	// Check if skopeo is available
	try {
		execSync("skopeo --version", { stdio: "pipe" });
	} catch (error) {
		throw new Error("skopeo is not available. Please install skopeo to run OCI converter tests.");
	}
	
	// Check if umoci is available
	try {
		execSync("umoci --version", { stdio: "pipe" });
	} catch (error) {
		throw new Error("umoci is not available. Please install umoci to run OCI converter tests.");
	}
});

afterAll(async () => {
	// Cleanup test directory
	await rm(TEST_DIR, { recursive: true, force: true });
	
	// Remove test Docker image
	try {
		execSync(`docker rmi ${TEST_IMAGE_NAME}`, { stdio: "pipe" });
	} catch (error) {
		// Ignore errors when removing image
	}
});

test("createTestDockerImage builds and saves Docker image", async () => {
	const dockerTarPath = await createTestDockerImage();
	
	// Verify the tar file was created
	const stats = await import("fs/promises").then(fs => fs.stat(dockerTarPath));
	expect(stats.isFile()).toBe(true);
	expect(stats.size).toBeGreaterThan(0);
	
	console.log(`Test Docker image created: ${dockerTarPath} (${stats.size} bytes)`);
}, 60000);


test("convertDockerTarToOCIBundle converts Docker tar to OCI bundle", async () => {
	const dockerTarPath = await createTestDockerImage();
	
	const result = await convertDockerTarToOCIBundle(dockerTarPath);
	
	try {
		// Verify the OCI bundle tar was created
		const stats = await import("fs/promises").then(fs => fs.stat(result.bundleTarPath));
		expect(stats.isFile()).toBe(true);
		expect(stats.size).toBeGreaterThan(0);
		
		console.log(`OCI bundle created: ${result.bundleTarPath} (${stats.size} bytes)`);
		
		// Validate the OCI bundle structure
		await validateOCIBundle(result.bundleTarPath);
		
	} finally {
		await result.cleanup();
	}
}, 120000);

test("full workflow: Docker image -> direct OCI bundle conversion", async () => {
	// Create test Docker image
	const dockerTarPath = await createTestDockerImage();
	
	// Convert directly to OCI bundle (like the new simplified flow)
	const convertResult = await convertDockerTarToOCIBundle(dockerTarPath);
	
	try {
		// Verify the final OCI bundle
		const stats = await import("fs/promises").then(fs => fs.stat(convertResult.bundleTarPath));
		expect(stats.isFile()).toBe(true);
		expect(stats.size).toBeGreaterThan(0);
		
		// Validate OCI bundle structure
		await validateOCIBundle(convertResult.bundleTarPath);
		
		console.log(`Full workflow completed successfully: ${convertResult.bundleTarPath}`);
		
	} finally {
		await convertResult.cleanup();
	}
}, 180000);