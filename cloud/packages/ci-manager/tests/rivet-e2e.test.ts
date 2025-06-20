import { test, expect, beforeAll, afterAll } from "vitest";
import { execSync } from "child_process";
import { mkdir, rm } from "fs/promises";
import { RivetClient } from "@rivet-gg/api";
import {
	createTestWebServerContext,
	pollBuildStatus,
	waitForActorReady,
	testActorEndpoint,
	createActorFromBuild,
} from "./test-utils";
import { type RivetUploadConfig } from "../src/rivet-uploader";

const TEST_DIR = "/tmp/rivet-test";

let rivetConfig: RivetUploadConfig;
let testActorIds: string[] = [];

async function findCIManagerActor(
	client: RivetClient,
	projectId: string,
): Promise<string | null> {
	try {
		const actorsResponse = await client.actors.list({
			project: projectId,
			environment: "ci",
			tagsJson: JSON.stringify({ name: "ci-manager" }),
		});

		const manager = actorsResponse.actors?.find(
			(actor) => actor.tags?.name === "ci-manager",
		);

		return manager?.id || null;
	} catch (error) {
		console.error("Error finding CI manager actor:", error);
		return null;
	}
}

async function createCIManagerActor(
	client: RivetClient,
	projectId: string,
	token: string,
): Promise<string> {
	const { builds } = await client.builds.list({
		project: projectId,
		environment: "ci",
		tagsJson: JSON.stringify({ name: "ci-runner", current: "true" }),
	});
	if (!builds[0]) throw new Error("Missing ci-runner build");

	const createResponse = await client.actors.create({
		project: projectId,
		environment: "ci",
		body: {
			tags: {
				name: "ci-manager",
			},
			buildTags: {
				name: "ci-manager",
				current: "true",
			},
			runtime: {
				environment: {
					KANIKO_EXECUTION_MODE: "rivet",
					KANIKO_BUILD_ID: builds[0].id,
					RIVET_CLOUD_TOKEN: token,
					RIVET_PROJECT: projectId,
					RIVET_ENVIRONMENT: "ci",
				},
			},
			network: {
				ports: {
					http: { protocol: "https", internalPort: 3000 },
				},
				waitReady: true,
			},
			resources: {
				cpu: 1000,
				memory: 1024,
			},
			lifecycle: {
				killTimeout: 30000,
				durable: true,
			},
		},
	});

	return createResponse.actor.id;
}

async function destroyActor(
	actorId: string,
	config: RivetUploadConfig,
): Promise<void> {
	const client = new RivetClient({ token: config.token });

	try {
		await client.actors.destroy(actorId, {
			project: config.project,
			environment: config.environment,
		});

		console.log(`Actor destroyed: ${actorId}`);
	} catch (error) {
		throw new Error(`Failed to destroy actor: ${error}`);
	}
}

beforeAll(async () => {
	await mkdir(TEST_DIR, { recursive: true });

	const token = process.env.RIVET_CLOUD_TOKEN;
	const project = process.env.RIVET_PROJECT;
	const environment = process.env.RIVET_ENVIRONMENT;

	if (!token || !project || !environment) {
		throw new Error(
			"Missing required environment variables: RIVET_CLOUD_TOKEN, RIVET_PROJECT, RIVET_ENVIRONMENT",
		);
	}

	rivetConfig = { token, project, environment };

	try {
		execSync("docker --version", { stdio: "pipe" });
	} catch (error) {
		throw new Error(
			"Docker is not available. Please install Docker to run Rivet tests.",
		);
	}
});

afterAll(async () => {
	for (const actorId of testActorIds) {
		try {
			await destroyActor(actorId, rivetConfig);
			console.log(`Cleaned up test actor: ${actorId}`);
		} catch (error) {
			console.warn(`Failed to cleanup test actor ${actorId}:`, error);
		}
	}

	await rm(TEST_DIR, { recursive: true, force: true });
});

test("full e2e", async () => {
	const token = rivetConfig.token;
	const projectId = rivetConfig.project;
	const environmentName = rivetConfig.environment;

	const client = new RivetClient({ token });

	let ciManagerActorId = await findCIManagerActor(client, projectId);

	if (!ciManagerActorId) {
		console.log("Creating CI manager actor...");
		ciManagerActorId = await createCIManagerActor(client, projectId, token);
		console.log(`Created CI manager actor: ${ciManagerActorId}`);

		await new Promise((resolve) => setTimeout(resolve, 10000));
	} else {
		console.log(`Using existing CI manager actor: ${ciManagerActorId}`);
	}

	const actorResponse = await client.actors.get(ciManagerActorId, {
		project: projectId,
		environment: "ci",
	});

	const endpoint = actorResponse.actor.network?.ports?.["http"]?.hostname;
	if (!endpoint) {
		throw new Error("Actor endpoint not available");
	}

	const serverUrl = `https://${endpoint}`;
	console.log(`Using CI manager endpoint: ${serverUrl}`);

	const contextBuffer = await createTestWebServerContext();
	const buildName = "rivet-test-build";

	const formData = new FormData();
	formData.append("buildName", buildName);
	formData.append("dockerfilePath", "configs/build.dockerfile");
	formData.append("environmentId", rivetConfig.environment);
	formData.append("context", new Blob([contextBuffer]), "context.tar.gz");

	const response = await fetch(
		`${serverUrl}/builds?serverUrl=${encodeURIComponent(serverUrl)}`,
		{
			method: "POST",
			body: formData,
		},
	);
	if (!response.ok) {
		console.log(
			"Response status",
			response.statusText,
			await response.text(),
		);
	}

	expect(response.ok).toBe(true);
	const result = await response.json();
	expect(result.buildId).toBeDefined();

	const buildId = result.buildId;
	console.log(`Started build: ${buildId}`);

	console.log("Polling build status...");
	const buildResult = await pollBuildStatus(serverUrl, buildId);
	expect(buildResult.status).toBe("success");
	console.log(`Build completed with status: ${buildResult.status}`);

	console.log("Creating actor from build...");
	const actorResult = await createActorFromBuild(
		buildResult.buildId,
		buildName,
		rivetConfig,
	);

	const actorId = actorResult.actorId;
	testActorIds.push(actorId);
	expect(actorResult.endpoint).toBeDefined();

	console.log(`Actor created: ${actorId}`);
	console.log(`Actor endpoint: ${actorResult.endpoint}`);

	await waitForActorReady(actorResult.endpoint);

	console.log("Testing actor HTTP endpoint...");
	const testResponse = await testActorEndpoint(actorResult.endpoint);

	expect(testResponse).toBeDefined();
	expect(testResponse.message).toBe("Rivet test successful!");
	expect(testResponse.timestamp).toBeDefined();
	expect(testResponse.hostname).toBeDefined();

	console.log("Actor HTTP test successful:", response);

	console.log("✅ Full E2E test completed successfully");
}, 600000);
