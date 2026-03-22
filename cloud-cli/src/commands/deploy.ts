/**
 * `rivet-cloud deploy` — build a Docker image, push it to the Rivet registry,
 * and update the managed pool configuration.
 *
 * Flow (mirrors rivet-dev/deploy-action):
 *   1. Inspect the cloud token → get org + project
 *   2. Ensure the target namespace exists (create if absent)
 *   3. `docker login` to the Rivet registry using the cloud token
 *   4. `docker build` the local context
 *   5. `docker tag` + `docker push` the image
 *   6. `managedPools.upsert` with the new image reference
 */

import type { Command } from "commander";
import task from "tasuku";
import { RivetError } from "@rivet-gg/cloud";
import { createCloudClient, type DockerCredentials } from "../lib/client.ts";
import { resolveToken } from "../lib/auth.ts";
import {
	assertDockerAvailable,
	dockerBuild,
	dockerLogin,
	dockerTagAndPush,
} from "../lib/docker.ts";
import { colors, detail, fatal, header } from "../utils/output.ts";

export interface DeployOptions {
	token?: string;
	namespace: string;
	pool: string;
	context: string;
	dockerfile?: string;
	tag?: string;
	minCount: string;
	maxCount: string;
	env?: string[];
	command?: string;
	args?: string;
	apiUrl?: string;
	platform?: string;
}

const IMAGE_ID_DISPLAY_LENGTH = 19;

export function registerDeployCommand(program: Command): void {
	program
		.command("deploy")
		.description("Build a Docker image and deploy it to a Rivet Cloud managed pool")
		.option("-t, --token <token>", "Cloud API token (overrides RIVET_CLOUD_TOKEN)")
		.option("-n, --namespace <name>", "Target namespace (created if absent)", "production")
		.option("-p, --pool <name>", "Managed pool name", "default")
		.option("--context <path>", "Docker build context directory", ".")
		.option("-f, --dockerfile <path>", "Path to Dockerfile")
		.option("--tag <tag>", "Docker image tag (defaults to git short SHA or timestamp)")
		.option("--min-count <n>", "Minimum runner instances", "1")
		.option("--max-count <n>", "Maximum runner instances", "5")
		.option(
			"-e, --env <KEY=VALUE>",
			"Environment variable (repeatable)",
			(val: string, prev: string[]) => [...(prev ?? []), val],
		)
		.option("--command <cmd>", "Override container entrypoint command")
		.option("--args <args>", "Space-separated args passed to the command")
		.option("--platform <platform>", "Docker build platform (e.g. linux/amd64)", "linux/amd64")
		.option("--api-url <url>", "Cloud API base URL", "https://cloud-api.rivet.dev")
		.action(async (opts: DeployOptions) => {
			await runDeploy(opts);
		});
}

async function runDeploy(opts: DeployOptions): Promise<void> {
	const token = resolveToken(opts.token);
	const client = createCloudClient({ token, baseUrl: opts.apiUrl });

	console.log(`\n${colors.accentBold("▶")} ${colors.label("Rivet Cloud Deploy")}\n`);

	// Step 1: Inspect token
	let org: string;
	let project: string;

	await task("Authenticating", async ({ setTitle }) => {
		const identity = await client.apiTokens.inspect().catch((err) => {
			fatal(
				`Authentication failed: ${err instanceof Error ? err.message : String(err)}`,
				"Check that RIVET_CLOUD_TOKEN is valid and not expired.",
			);
		});
		org = identity.organization;
		project = identity.project;
		setTitle(`Authenticated (org: ${org}, project: ${project})`);
	});

	// Step 2: Ensure namespace exists
	await task(`Namespace: ${opts.namespace}`, async ({ setTitle }) => {
		let exists = true;
		try {
			await client.namespaces.get(project!, opts.namespace, { org: org! });
		} catch (err) {
			if (err instanceof RivetError && err.statusCode === 404) {
				exists = false;
			} else {
				throw err;
			}
		}

		if (!exists) {
			await client.namespaces.create(project!, { displayName: opts.namespace, org: org! });
			setTitle(`Created namespace "${opts.namespace}"`);
		} else {
			setTitle(`Namespace "${opts.namespace}" ready`);
		}
	});

	// Step 3: Docker availability
	await assertDockerAvailable();

	// Use the cloud token directly as Docker registry credentials.
	// The Rivet registry accepts cloud tokens as the password.
	const creds: DockerCredentials = {
		registryUrl: "registry.rivet.dev",
		username: "token",
		password: token,
	};

	// Step 4: Docker login
	await task(`Logging in to ${creds.registryUrl}`, async () => {
		await dockerLogin(creds);
	});

	// Step 5: Build Docker image
	let imageId: string;
	await task("Building Docker image", async ({ setTitle }) => {
		const result = await dockerBuild({
			context: opts.context,
			dockerfile: opts.dockerfile,
			platform: opts.platform,
		});
		imageId = result.imageId;
		setTitle(`Built image ${imageId!.slice(0, IMAGE_ID_DISPLAY_LENGTH)}`);
	});

	// Derive image tag
	const imageTag = opts.tag ?? (await resolveImageTag());
	const repository = `${project!}/${opts.pool}`;
	const remoteRef = `${creds.registryUrl}/${org!}/${repository}:${imageTag}`;

	// Step 6: Tag and push
	await task(`Pushing ${remoteRef}`, async () => {
		await dockerTagAndPush({ localImageId: imageId!, remoteRef });
	});

	// Step 7: Parse environment variables
	const envVars = parseEnvVars(opts.env);

	// Step 8: Upsert managed pool
	await task(`Updating managed pool "${opts.pool}"`, async ({ setTitle }) => {
		await client.managedPools.upsert(project!, opts.namespace, opts.pool, {
			org: org!,
			image: { repository, tag: imageTag },
			minCount: Number(opts.minCount),
			maxCount: Number(opts.maxCount),
			...(Object.keys(envVars).length > 0 ? { environment: envVars } : {}),
			...(opts.command ? { command: opts.command } : {}),
			...(opts.args ? { args: opts.args.split(" ") } : {}),
		});
		setTitle(`Pool "${opts.pool}" updated`);
	});

	console.log("");
	header("Deployment complete");
	detail("Namespace", opts.namespace);
	detail("Pool", opts.pool);
	detail("Image", `${repository}:${imageTag}`);
	detail(
		"Dashboard",
		`https://hub.rivet.dev/orgs/${org!}/projects/${project!}/ns/${opts.namespace}`,
	);
	console.log("");
}

async function resolveImageTag(): Promise<string> {
	try {
		const result = await Bun.$`git rev-parse --short HEAD`.quiet().text();
		return result.trim();
	} catch {
		return `deploy-${Date.now()}`;
	}
}

function parseEnvVars(raw: string[] | undefined): Record<string, string> {
	if (!raw?.length) return {};
	const out: Record<string, string> = {};
	for (const entry of raw) {
		const eq = entry.indexOf("=");
		if (eq === -1) {
			fatal(`Invalid --env value "${entry}". Expected KEY=VALUE format.`);
		}
		const key = entry.slice(0, eq);
		const value = entry.slice(eq + 1);
		out[key] = value;
	}
	return out;
}

