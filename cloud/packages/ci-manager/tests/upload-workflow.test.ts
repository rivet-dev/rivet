import { test, expect, beforeAll, afterAll } from "vitest";
import { mkdir, rm } from "fs/promises";
import { convertDockerTarToOCIBundle } from "../src/oci-converter";
import {
	uploadOCIBundleToRivet,
	type RivetUploadConfig,
} from "../src/rivet-uploader";
import { RivetClient } from "@rivet-gg/api";
import {
	createTestWebServerImage,
	waitForActorReady,
	testActorEndpoint,
} from "./test-utils";

const TEST_DIR = "/tmp/upload-test";

let rivetConfig: RivetUploadConfig;
let testActorIds: string[] = [];

async function createRivetActor(
	buildId: string,
	buildName: string,
	config: RivetUploadConfig,
): Promise<{ actorId: string; endpoint?: string }> {
	const client = new RivetClient({ token: config.token });

	try {
		const response = await client.actors.create({
			project: config.project,
			environment: config.environment,
			body: {
				tags: { name: buildName },
				buildTags: {
					name: buildName,
					current: "true",
				},
				network: {
					ports: {
						http: {
							protocol: "https",
							internalPort: 3000,
						},
					},
				},
				resources: {
					cpu: 100,
					memory: 128,
				},
			},
		});

		const actorId = response.actor.id;

		await new Promise((resolve) => setTimeout(resolve, 5000));

		const actorDetails = await client.actors.get(actorId, {
			project: config.project,
			environment: config.environment,
		});

		const endpoint = actorDetails.actor.network?.ports?.http?.hostname;

		return {
			actorId,
			endpoint: endpoint ? `https://${endpoint}` : undefined,
		};
	} catch (error) {
		throw new Error(`Failed to create Rivet actor: ${error}`);
	}
}

async function destroyRivetActor(
	actorId: string,
	config: RivetUploadConfig,
): Promise<void> {
	const client = new RivetClient({ token: config.token });

	try {
		await client.actors.destroy(actorId, {
			project: config.project,
			environment: config.environment,
		});

		console.log(`Rivet actor destroyed: ${actorId}`);
	} catch (error) {
		throw new Error(`Failed to destroy Rivet actor: ${error}`);
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

	rivetConfig = { token, project, environment: "staging" };
});

afterAll(async () => {
	for (const actorId of testActorIds) {
		try {
			await destroyRivetActor(actorId, rivetConfig);
			console.log(`Cleaned up test actor: ${actorId}`);
		} catch (error) {
			console.warn(`Failed to cleanup test actor ${actorId}:`, error);
		}
	}

	await rm(TEST_DIR, { recursive: true, force: true });
});

test("upload workflow", async () => {
	const { dockerTarPath, imageName } = await createTestWebServerImage();
	const conversionResult = await convertDockerTarToOCIBundle(dockerTarPath);

	let actorId: string | undefined;

	try {
		console.log("Uploading OCI bundle to Rivet...");
		const uploadResult = await uploadOCIBundleToRivet(
			conversionResult.bundleTarPath,
			"rivet-upload-test",
			imageName,
			rivetConfig,
			"2.0.0",
		);

		console.log(`Build uploaded: ${uploadResult.buildId}`);

		console.log("Creating Rivet actor...");
		const actorResult = await createRivetActor(
			uploadResult.buildId,
			"rivet-upload-test",
			rivetConfig,
		);

		actorId = actorResult.actorId;
		testActorIds.push(actorId);
		expect(actorResult.endpoint).toBeDefined();

		console.log(`Actor created: ${actorId}`);
		console.log(`Actor endpoint: ${actorResult.endpoint}`);

		await waitForActorReady(actorResult.endpoint!);

		console.log("Testing actor HTTP endpoint...");
		const response = await testActorEndpoint(actorResult.endpoint!);

		expect(response).toBeDefined();
		expect(response.message).toBe("Rivet test successful!");
		expect(response.timestamp).toBeDefined();
		expect(response.hostname).toBeDefined();

		console.log("Actor HTTP test successful:", response);

		console.log("✅ Upload workflow test completed successfully");
	} finally {
		await conversionResult.cleanup();
	}
}, 600000);