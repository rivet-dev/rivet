import { RivetClient } from "@rivetkit/engine-api-full";
import { FreestyleSandboxes } from "freestyle-sandboxes";
import { prepareDirForDeploymentSync } from "freestyle-sandboxes/utils";
import { writeFile, cp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { $ } from "execa";

export type LogCallback = (message: string) => Promise<void>;

export interface CloudDeployRequest {
	cloudEndpoint: string;
	cloudToken: string;
	engineEndpoint: string;
}

export interface SelfHostedDeployRequest {
	endpoint: string;
	token: string;
}

export interface DeployRequest {
	registryCode: string;
	appCode: string;
	datacenter?: string;
	freestyleDomain: string;
	freestyleApiKey: string;
	kind:
		| { cloud: CloudDeployRequest }
		| { selfHosted: SelfHostedDeployRequest };
}

/** Assemble the repository that we're going to deploy to Freestyle. */
export async function setupRepo(config: {
	registryCode: string;
	appCode: string;
	log: LogCallback;
}): Promise<string> {
	await config.log("Preparing project files");
	const tmpDir = join(tmpdir(), `rivet-deploy-${Date.now()}`);
	const templateDir = join(process.cwd(), "template");

	await cp(templateDir, tmpDir, { recursive: true });

	await writeFile(join(tmpDir, "src/backend/registry.ts"), config.registryCode);
	await writeFile(join(tmpDir, "src/frontend/App.tsx"), config.appCode);

	return tmpDir;
}

/** Build frontend with Vite. */
export async function buildFrontend(
	projectDir: string,
	envVars: Record<string, string>,
	log: LogCallback,
) {
	await log("Installing dependencies");
	await $({ cwd: projectDir })`pnpm install --no-frozen-lockfile`;

	await log("Building frontend");
	await $({ cwd: projectDir, env: { ...process.env, ...envVars } })`pnpm run build:frontend`;
}

/**
 * Deploy the application to Freestyle Sandboxes.
 *
 * Sets up the project directory and deploys the backend to Freestyle.
 */
export async function deployToFreestyle(config: {
	registryCode: string;
	appCode: string;
	domain: string;
	apiKey: string;
	envVars: Record<string, string>;
	log: LogCallback;
}): Promise<{ deploymentId: string }> {
	const projectDir = await setupRepo({
		registryCode: config.registryCode,
		appCode: config.appCode,
		log: config.log,
	});

	await buildFrontend(projectDir, config.envVars, config.log);

	await config.log("Deploying to Freestyle");

	const freestyle = new FreestyleSandboxes({
		apiKey: config.apiKey,
	});

	const deploymentSource = prepareDirForDeploymentSync(projectDir);

	const result = await freestyle.deployWeb(deploymentSource, {
		envVars: {
			LOG_LEVEL: "debug",
			FREESTYLE_ENDPOINT: `https://${config.domain}`,
			RIVET_RUNNER_KIND: "serverless",
			...config.envVars,
		},
		timeout: 60 * 5,
		entrypoint: "src/backend/server.ts",
		domains: [config.domain],
		build: false,
	});

	return { deploymentId: result.deploymentId };
}

/**
 * Configure a serverless runner in Rivet for a specific datacenter.
 */
export async function configureRivetServerless(config: {
	rivet: RivetClient;
	domain: string;
	namespace: string;
	datacenter?: string;
	log: LogCallback;
}) {
	const datacenter = config.datacenter || "us-west-1";
	await config.log(`Configuring runner in ${datacenter}`);

	await config.rivet.runnerConfigsUpsert("default", {
		datacenters: {
			[datacenter]: {
				serverless: {
					url: `https://${config.domain}/api/rivet`,
					headers: {},
					runnersMargin: 0,
					minRunners: 0,
					maxRunners: 1_000,
					slotsPerRunner: 1,
					requestLifespan: 60 * 5,
				},
			},
		},
		namespace: config.namespace,
	});
}

export function generateNamespaceName(): string {
	return `ns-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`;
}
