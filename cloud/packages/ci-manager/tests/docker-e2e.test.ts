/**
 * This test is intended to check that Kaniko works as intended with an inline driver (not using Rivet).
 *
 * Useful for quick iteration on the server.
 */

import { describe, it, expect, beforeAll, afterAll, test } from "vitest";
import { execSync } from "node:child_process";
import { join } from "node:path";
import { writeFile } from "fs/promises";
import getPort from "get-port";
import { createServer } from "../src/server";
import { RivetClient } from "@rivet-gg/api";
import {
	createSampleDockerContext,
	createFailingDockerContext,
	createTestWebServerContext,
	createBuildWithContext,
	getBuildStatus,
	pollBuildStatus,
	downloadOutputTar,
	waitForActorReady,
	testActorEndpoint,
	createActorFromBuild,
} from "./test-utils";
import { convertDockerTarToOCIBundle } from "../src/oci-converter";
import {
	uploadOCIBundleToRivet,
	type RivetUploadConfig,
} from "../src/rivet-uploader";

describe("Docker", () => {
	let server: any;
	let baseUrl: string;
	let rivetConfig: RivetUploadConfig = undefined as any;
	let testActorIds: string[] = [];

	beforeAll(async () => {
		// Build the ci-manager dockerfile before starting tests
		console.log("Building ci-runner Docker image...");
		execSync("docker build -t ci-runner .", {
			cwd: join(__dirname, "../../ci-runner"),
			stdio: "inherit",
		});
		console.log("ci-runner Docker image built successfully");

		const port = await getPort();
		server = await createServer(port);
		baseUrl = `http://localhost:${port}`;

		// Initialize Rivet config if environment variables are available
		const token = process.env.RIVET_CLOUD_TOKEN;
		const project = process.env.RIVET_PROJECT;
		const environment = process.env.RIVET_ENVIRONMENT;

		if (token && project && environment) {
			rivetConfig = { token, project, environment };
			console.log("Rivet config initialized - will test actor creation");
		} else {
			throw new Error(
				"Rivet config not available - skipping actor creation tests",
			);
		}
	});

	afterAll(async () => {
		// Clean up test actors
		const client = new RivetClient({ token: rivetConfig.token });
		for (const actorId of testActorIds) {
			try {
				await client.actors.destroy(actorId, {
					project: rivetConfig.project,
					environment: rivetConfig.environment,
				});
				console.log(`Cleaned up test actor: ${actorId}`);
			} catch (error) {
				console.warn(`Failed to cleanup test actor ${actorId}:`, error);
			}
		}

		if (server) {
			if (typeof server.stop === "function") {
				server.stop();
			} else if (typeof server.close === "function") {
				server.close();
			}
		}
	});

	it("full e2e", async () => {
		console.log("Starting E2E CI Manager Test");

		// Create sample Docker context with web server
		console.log("Creating sample Docker context with web server...");
		const contextBuffer = await createTestWebServerContext();
		console.log(`Context created (${contextBuffer.length} bytes)`);
		expect(contextBuffer.length).toBeGreaterThan(0);

		// Create build with context
		console.log("Creating build with context...");
		const buildName = "test-build-" + Date.now();
		const buildId = await createBuildWithContext(
			baseUrl,
			buildName,
			"configs/build.dockerfile",
			contextBuffer,
			rivetConfig.environment,
		);
		console.log(`Build created and started: ${buildId}`);
		expect(buildId).toBeDefined();
		expect(typeof buildId).toBe("string");

		// Poll build status until completion
		console.log("Polling build status...");
		const buildResult = await pollBuildStatus(baseUrl, buildId);
		console.log(`Build completed with status: ${buildResult.status}`);

		// The build should actually succeed since we have Kaniko available
		expect(buildResult.status).toBe("success");

		console.log("Build completed successfully!");

		// Get final build status
		console.log("Checking final build status:");
		const buildStatus = await getBuildStatus(baseUrl, buildId);
		console.log(buildStatus);
		expect(buildStatus.id).toBe(buildId);
		expect(buildStatus.status.type).toBe("success");

		let actorId: string | undefined;

		// Create actor from build
		console.log("Creating actor from build...");
		const actorResult = await createActorFromBuild(
			buildResult.buildId,
			buildName,
			rivetConfig,
		);

		actorId = actorResult.actorId;
		testActorIds.push(actorId);

		expect(actorResult.endpoint).toBeDefined();

		console.log(`Actor created: ${actorId}`);
		console.log(`Actor endpoint: ${actorResult.endpoint}`);

		// Wait for actor to be ready and test endpoint
		console.log("Testing actor HTTP endpoint...");
		const testResponse = await testActorEndpoint(actorResult.endpoint);

		expect(testResponse).toBeDefined();
		expect(testResponse.message).toBe("Rivet test successful!");
		expect(testResponse.timestamp).toBeDefined();
		expect(testResponse.hostname).toBeDefined();

		console.log("E2E Test completed successfully!");
	}, 600000); // Increase timeout to 10 minutes for the full workflow including Rivet

	it("should handle build not found", async () => {
		const response = await fetch(`${baseUrl}/builds/non-existent-build`);
		expect(response.status).toBe(404);

		const result = await response.json();
		expect(result.error).toBe("Build not found");
	});

	it("should reject build creation with missing data", async () => {
		const formData = new FormData();
		formData.append("buildName", "test-build");
		// Missing dockerfilePath and context file

		const response = await fetch(`${baseUrl}/builds`, {
			method: "POST",
			body: formData,
		});

		expect(response.status).toBe(400);

		const result = await response.json();
		expect(result.error).toBe("dockerfilePath is required");
	});

	it("should handle failing dockerfile build correctly", async () => {
		console.log("Starting failing dockerfile test");

		// Create failing Docker context
		console.log("Creating failing Docker context...");
		const contextBuffer = await createFailingDockerContext();
		console.log(`Failing context created (${contextBuffer.length} bytes)`);
		expect(contextBuffer.length).toBeGreaterThan(0);

		// Create build with failing context
		console.log("Creating build with failing context...");
		const buildName = "test-failing-build-" + Date.now();
		const buildId = await createBuildWithContext(
			baseUrl,
			buildName,
			"configs/fail.dockerfile",
			contextBuffer,
			rivetConfig.environment,
		);
		console.log(`Failing build created: ${buildId}`);
		expect(buildId).toBeTruthy();

		// Poll build status until failure
		console.log("Polling build status for failure...");
		const buildResult = await pollBuildStatus(baseUrl, buildId);
		console.log(`Build completed with status: ${buildResult.status}`);

		// Verify build completed with failure status
		expect(buildResult.status).toBe("failure");

		// Check final build status via API
		console.log("Checking final build status:");
		const buildStatus = await getBuildStatus(baseUrl, buildId);
		console.log(buildStatus);
		expect(buildStatus.id).toBe(buildId);
		expect(buildStatus.status.type).toBe("failure");

		// Get final build status to verify failure reason
		const finalBuildStatus = await getBuildStatus(baseUrl, buildId);
		expect(finalBuildStatus.status.data.reason).toBeTruthy();

		console.log("Failing dockerfile test completed successfully!");
	}, 70000); // 70 second timeout for failing build test
});
