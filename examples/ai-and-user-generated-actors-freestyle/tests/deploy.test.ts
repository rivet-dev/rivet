import { readFileSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { deployWithRivetCloud } from "../src/backend/deploy-with-rivet-cloud";
import { deployWithRivetSelfHosted } from "../src/backend/deploy-with-rivet-self-hosted";
import type { DeployRequest, LogCallback } from "../src/backend/utils";

// Simple log callback for tests
const testLog: LogCallback = async (message: string) => {
	console.log(message);
};

// Load sample code from template files
const SAMPLE_REGISTRY_CODE = readFileSync(
	join(process.cwd(), "template/src/backend/registry.ts"),
	"utf-8",
);

const SAMPLE_APP_CODE = readFileSync(
	join(process.cwd(), "template/src/frontend/App.tsx"),
	"utf-8",
);

describe("Deploy Functions", () => {
	// Load environment variables
	beforeAll(() => {
		// Check if required environment variables are set
		const requiredEnvVars = ["FREESTYLE_DOMAIN", "FREESTYLE_API_KEY"];

		for (const envVar of requiredEnvVars) {
			if (!process.env[envVar]) {
				throw new Error(
					`Missing required environment variable: ${envVar}`,
				);
			}
		}
	});

	describe("deployWithRivetCloud", () => {
		it.skip("should deploy successfully with Rivet Cloud", async () => {
			// Skip this test if cloud credentials are not provided
			if (
				!process.env.RIVET_CLOUD_ENDPOINT ||
				!process.env.RIVET_CLOUD_TOKEN ||
				!process.env.RIVET_ENGINE_ENDPOINT
			) {
				console.log(
					"Skipping cloud deployment test - missing credentials",
				);
				return;
			}

			const request: DeployRequest = {
				registryCode: SAMPLE_REGISTRY_CODE,
				appCode: SAMPLE_APP_CODE,
				datacenter: process.env.RIVET_DATACENTER,
				freestyleDomain: process.env.FREESTYLE_DOMAIN!,
				freestyleApiKey: process.env.FREESTYLE_API_KEY!,
				kind: {
					cloud: {
						cloudEndpoint: process.env.RIVET_CLOUD_ENDPOINT,
						cloudToken: process.env.RIVET_CLOUD_TOKEN,
						engineEndpoint: process.env.RIVET_ENGINE_ENDPOINT,
					},
				},
			};

			const result = await deployWithRivetCloud(request, testLog);

			expect(result).toBeDefined();
			expect(result.success).toBe(true);
			expect(result.tokens).toBeDefined();
			expect(result.tokens.runnerToken).toBeDefined();
			expect(result.tokens.publishableToken).toBeDefined();
		}, 300000); // 5 minute timeout for deployment
	});

	describe("deployWithRivetSelfHosted", () => {
		it.skip("should deploy successfully with Rivet Self-Hosted", async () => {
			// Skip this test if self-hosted credentials are not provided
			if (!process.env.RIVET_ENDPOINT || !process.env.RIVET_TOKEN) {
				console.log(
					"Skipping self-hosted deployment test - missing credentials",
				);
				return;
			}

			const request: DeployRequest = {
				registryCode: SAMPLE_REGISTRY_CODE,
				appCode: SAMPLE_APP_CODE,
				datacenter: process.env.RIVET_DATACENTER,
				freestyleDomain: process.env.FREESTYLE_DOMAIN!,
				freestyleApiKey: process.env.FREESTYLE_API_KEY!,
				kind: {
					selfHosted: {
						endpoint: process.env.RIVET_ENDPOINT,
						token: process.env.RIVET_TOKEN,
					},
				},
			};

			const result = await deployWithRivetSelfHosted(request, testLog);

			expect(result).toBeDefined();
			expect(result.success).toBe(true);
		}, 300000); // 5 minute timeout for deployment
	});
});
